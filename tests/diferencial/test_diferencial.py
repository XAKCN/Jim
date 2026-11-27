import getpass
from hashlib import sha256
from inspect import signature
import json
import socket
import unittest
from unittest.mock import patch

from jim import _core

from . import vigia
from .formato import DIRETORIO, carregar_casos, decodificar, etiquetar, textos
from .limites import CAMINHO_PRIVADO, GRUPOS, LIMITES


def setUpModule():
    if _core.IMPLEMENTADO:
        vigia.instalar()


def codigo(mensagem):
    return mensagem.split(": ", 1)[0] if isinstance(mensagem, str) else mensagem


def normalizar(valor):
    if type(valor) is list:
        return [normalizar(item) for item in valor]
    if type(valor) is dict:
        resultado = {}
        for chave, item in valor.items():
            if chave in ("motivo", "mensagem", "erro"):
                resultado[chave] = codigo(item)
            elif chave == "erros" and type(item) is list:
                resultado[chave] = [codigo(erro) for erro in item]
            else:
                resultado[chave] = normalizar(item)
        return resultado
    return valor


class CasosTest(unittest.TestCase):
    def test_frescor_e_cobertura(self):
        meta = json.loads((DIRETORIO / "ORACULO.json").read_text(encoding="ascii"))
        self.assertEqual(set(meta), {"tag", "commit", "python", "unicode", "gerador_sha256", "limites"})
        self.assertEqual((meta["tag"], meta["commit"]), ("v5-alpha", "f761213"))
        gerador = DIRETORIO.parent / "gerar.py"
        self.assertEqual(meta["gerador_sha256"], sha256(gerador.read_bytes()).hexdigest())
        self.assertEqual(set(meta["limites"]), LIMITES)
        casos = list(carregar_casos())
        self.assertEqual({l for c in casos for l in c["limite"]}, LIMITES)
        self.assertEqual(len({c["id"] for c in casos}), len(casos))
        for caso in casos:
            self.assertTrue(caso["id"].startswith(caso["grupo"]), caso["id"])
            self.assertTrue(all(l.startswith(caso["grupo"] + ": ") for l in caso["limite"]), caso["id"])
        self.assertEqual({c["grupo"] for c in casos}, set(GRUPOS))

    def test_privacidade(self):
        privados = (getpass.getuser(), socket.gethostname())
        self.assertEqual({p.name for p in DIRETORIO.glob("*.json")},
                         {"ORACULO.json", *(f"{g}.json" for g in GRUPOS.values())})
        for caminho in DIRETORIO.glob("*.json"):
            conteudo = caminho.read_text(encoding="ascii")
            objeto = decodificar(json.loads(conteudo))
            for texto in (conteudo, *textos(objeto)):
                self.assertFalse(CAMINHO_PRIVADO.search(texto), caminho.name)
                self.assertFalse(any(p and p in texto for p in privados), caminho.name)


class VigiaTest(unittest.TestCase):
    def test_requer_pass_dos_dois_lados_e_caso(self):
        def avaliar(a, b=None):
            return {"resultado": "PASS"}

        def saida(resultado):
            return {"tipo": "retorno", "valor": {"resultado": resultado}}

        nome = "avaliar_criterio_43"
        assinatura = signature(avaliar)
        casos = [
            {"funcao": nome, "args": {"a": "caso"}, "esperado": saida("PASS"),
             "oraculo": {"saida": saida("PASS")}},
            {"funcao": nome, "args": {"a": "divergente"}, "esperado": saida("PASS"),
             "oraculo": {"saida": saida("UNKNOWN")}},
        ]
        guarda = vigia.Vigia(casos, {nome: assinatura})
        chamada = guarda.envolver(nome, avaliar, assinatura)
        self.assertEqual(chamada("caso", b=None), saida("PASS")["valor"])
        with self.assertRaisesRegex(AssertionError, "^PASS INDEVIDO:"):
            chamada("divergente")
        with self.assertRaisesRegex(AssertionError, "^ACHADO CANDIDATO: PASS sem caso diferencial:"):
            chamada("ausente")

    def test_detecta_pass_indevido_em_parte_do_gate(self):
        esperado = {"veredito": "FAIL", "integracao": {"resultado": "UNKNOWN"}}
        obtido = {"veredito": "FAIL", "integracao": {"resultado": "PASS"}}
        self.assertEqual(vigia.pass_indevido(obtido, esperado), "integracao.resultado")

    def test_fecho_coberto_nao_autoriza_reproducao_divergente(self):
        def verificar(fecho_bytes, reproducao=None):
            return {"veredito": "PASS"}

        nome = "verificar_fecho"
        assinatura = signature(verificar)
        dados = b"fecho\n"
        digest = sha256(dados).hexdigest()
        casos = [
            {"funcao": nome, "args": etiquetar({"fecho_bytes": dados, "reproducao": rep}),
             "fecho_sha256": digest, "esperado": {"tipo": "retorno", "valor": {"veredito": veredito}},
             "oraculo": {"gate": {"veredito": "PASS"}}}
            for rep, veredito in (({"observacao": 1}, "PASS"), ({"observacao": 2}, "FAIL"))
        ]
        guarda = vigia.Vigia(casos, {nome: assinatura})
        verificar = guarda.envolver(nome, verificar, assinatura)
        guarda.registrar(("fecho", digest))
        with self.assertRaisesRegex(AssertionError, "^PASS INDEVIDO:"):
            verificar(dados, {"observacao": 2})


@unittest.skipIf(_core.IMPLEMENTADO is False, "diferencial pulado: jim._core.IMPLEMENTADO é False")
class DiferencialTest(unittest.TestCase):
    def test_contra_o_oraculo(self):
        for caso in carregar_casos():
            with self.subTest(caso=caso["id"], limites=caso["limite"]):
                self.comparar(caso)

    def test_comparador_recusa_retorno_divergente(self):
        caso = {
            "id": "retorno-divergente",
            "funcao": "json_canonico",
            "args": {"valor": True},
            "esperado": {"tipo": "retorno", "valor": etiquetar(b"false")},
            "comparar": "exato",
        }
        with patch.object(_core, "iguais", return_value=True):
            with self.assertRaisesRegex(AssertionError, "obtido="):
                self.comparar(caso)

    def comparar(self, caso):
        funcao = getattr(_core, caso["funcao"])
        esperado = caso["esperado"]
        try:
            valor = funcao(**decodificar(caso["args"]))
        except _core.NaoImplementado as erro:
            self.fail(f"NaoImplementado com núcleo habilitado: {erro}")
        except _core.EntradaInvalida as erro:
            obtido = {"tipo": "excecao", "classe": "EntradaInvalida", "mensagem": str(erro)}
        else:
            indevido = vigia.pass_indevido(valor, decodificar(esperado.get("valor")))
            if indevido:
                self.fail(f"PASS INDEVIDO: {caso['id']} {indevido}")
            if caso["funcao"] in (*vigia.GATES, *vigia.FECHOS) and vigia.passou(valor):
                self.assertTrue(vigia.oraculo_passou(caso), f"PASS INDEVIDO: {caso['id']} oráculo recusou")
            obtido = {"tipo": "retorno", "valor": etiquetar(valor)}
        if caso["comparar"] == "codigo":
            obtido, esperado = normalizar(obtido), normalizar(esperado)
        # Compara os valores JSON, incluindo as etiquetas dos campos binários.
        if obtido["tipo"] == esperado["tipo"] == "retorno":
            self.assertEqual(vigia.canonico(obtido["valor"]), vigia.canonico(esperado["valor"]),
                             f"obtido={obtido!r}\nesperado={esperado!r}")
        else:
            self.assertEqual(obtido, esperado)

    def test_ordem_insercao_e_ordem_codigo(self):
        # G5 ordena chaves no arquivo JSON, então o caso com ordem inversa é
        # construído aqui a partir dos vetores rejeitados pelo oráculo.
        for limite, chave_esperada in (("ordem de inserção R-2.12", "z"),
                                      ("duas dependências ausentes", "a")):
            with self.subTest(limite=limite):
                caso = next(c for c in carregar_casos() if f"G: {limite}" in c["limite"])
                args = decodificar(caso["args"])
                deps = args["registro"]["dependencias"]
                args["registro"]["dependencias"] = dict(reversed(list(deps.items())))
                saida = _core.avaliar_criterio_43(**args)
                self.assertEqual(saida["resultado"], "UNKNOWN")
                self.assertIn(f"'{chave_esperada}'", saida["motivo"])
