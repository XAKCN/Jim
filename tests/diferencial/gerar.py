"""Gera casos sintéticos usando a v5-alpha em modo somente leitura."""

from collections import defaultdict
from copy import deepcopy
from datetime import date
import getpass
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import socket
import subprocess
import sys
from tempfile import TemporaryDirectory, gettempdir
import unicodedata

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from tests.candidato import CONTRATO, SONDA, contrato_bytes
from tests.diferencial.formato import DIRETORIO, etiquetar, textos
from tests.diferencial.limites import CAMINHO_PRIVADO, GRUPOS, LIMITES


def sha(dados):
    return hashlib.sha256(dados).hexdigest()


def canonico(valor):
    return json.dumps(valor, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def selar(fecho):
    sem_selo = {k: v for k, v in fecho.items() if k != "selo"}
    return {**sem_selo, "selo": sha(canonico(sem_selo))}


def retorno(valor):
    return {"tipo": "retorno", "valor": valor}


def excecao(mensagem):
    return {"tipo": "excecao", "classe": "EntradaInvalida", "mensagem": mensagem}


def adaptar_contrato(c):
    return {"release": "0.0", **{k: v for k, v in c.items() if k != "esquema"}}


class Oraculo:
    def __init__(self, temporario, cadeia, gate43, release):
        self.arquivo = temporario / "cadeia.jsonl"
        self.cadeia = cadeia
        self.gate43 = gate43
        self.release = release

    def carregar(self, dados):
        try:
            texto = dados.decode("utf-8")
        except UnicodeDecodeError:
            raise ValueError("R-5.16: UTF-8 inválido") from None
        return self.cadeia._carregar(texto)

    def ultima(self, texto):
        return self.release.ultima_linha_json(texto.replace("\r\n", "\n").replace("\r", "\n"))

    def adaptar_execucao(self, execucao):
        def observado(ex):
            return {"exit_code": ex["exit_code"], "saida": self.ultima(ex["stdout"])}

        saida = observado(execucao)
        for chave in ("criterio", "comando"):
            if chave in execucao:
                saida[chave] = execucao[chave]
        if "controles_negativos" in execucao:
            controles = []
            for controle in execucao["controles_negativos"]:
                c = {"alterado": controle["alterado"], **observado(controle)}
                stderr = controle["stderr"]
                if stderr:
                    c.update(stderr_sha256=sha(stderr.encode()), stderr_final=stderr.strip().splitlines()[-5:])
                controles.append(c)
            saida["controles_negativos"] = controles
        return saida

    def gravada(self, ex):
        saida = {
            **ex,
            "stdout_sha256": sha(ex["stdout"].encode()),
            "stderr_sha256": sha(ex["stderr"].encode()),
            "saida": self.ultima(ex["stdout"]),
        }
        if "controles_negativos" in ex:
            saida["controles_negativos"] = [self.gravada(c) for c in ex["controles_negativos"]]
        return saida

    def registro(self, contrato, execucoes, executados, atuais, hc, hc_depois):
        a, i, c = self.release.registros(
            adaptar_contrato(contrato), [self.adaptar_execucao(e) for e in execucoes],
            executados, atuais, hc, hc_depois,
        )
        return {"avaliacoes": a, "integracao": i, "correntes": c}

    def gate(self, funcao, args):
        args = dict(args)
        hoje = args["hoje"]
        if isinstance(hoje, str) and re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}", hoje):
            try:
                args["hoje"] = date.fromisoformat(hoje)
            except ValueError:
                pass
        return getattr(self.gate43, funcao)(**args)

    def operar_cadeia(self, funcao, args):
        dados = args["dados"]
        self.arquivo.write_bytes(dados)
        ancora = args.get("ancora")
        if funcao == "proxima_linha":
            nova = self.cadeia.acrescentar(self.arquivo, args["evento"], ancora)
            return {"linha": self.arquivo.read_bytes()[len(dados):], "ancora": nova}
        metodo = self.cadeia.eventos if funcao == "eventos_cadeia" else self.cadeia.verificar
        return metodo(self.arquivo, ancora)

    def montar(self, contrato_bytes, caminho_contrato, execucoes, arquivos, arquivos_depois,
               contrato_sha256_depois, ambiente):
        contrato = self.carregar(contrato_bytes)
        hc = sha(contrato_bytes)
        candidato = "sha256:" + self.release.hash_arvore(arquivos)
        registros = self.registro(contrato, execucoes, arquivos, arquivos_depois, hc, contrato_sha256_depois)
        cg = self.release.contrato_gate(adaptar_contrato(contrato), candidato)
        gate = self.gate("gate_43", {"contrato": cg, **registros, "hoje": contrato["data_referencia"]})
        veredito = "UNPROVEN" if gate["veredito"] == "PASS" else gate["veredito"]
        fecho = {
            "esquema": "jim-fecho/1", "jim": "0.1.0",
            "contrato": {"caminho": caminho_contrato, "sha256": hc, "sha256_depois": contrato_sha256_depois,
                         "versao": contrato["versao"]},
            "data_referencia": contrato["data_referencia"], "candidato": candidato,
            "arquivos": arquivos, "arquivos_depois": arquivos_depois,
            "execucoes": [self.gravada(e) for e in execucoes], "evidencias": registros["avaliacoes"],
            "integracao": registros["integracao"], "correntes": registros["correntes"],
            "contrato_gate": cg, "gate": gate, "veredito": veredito, "ambiente": ambiente,
        }
        fecho = selar(fecho)
        dados = canonico(fecho) + b"\n"
        return {"fecho": fecho, "dados": dados, "sha256": sha(dados), "veredito": veredito}

    def chamar(self, funcao, args):
        if funcao in ("gate_43", "avaliar_criterio_43"):
            return "gate43." + funcao, self.capturar(lambda: self.gate(funcao, args))
        if funcao in ("verificar_cadeia", "eventos_cadeia", "proxima_linha"):
            nome = {"verificar_cadeia": "verificar", "eventos_cadeia": "eventos", "proxima_linha": "acrescentar"}
            return "cadeia." + nome[funcao], self.capturar(lambda: self.operar_cadeia(funcao, args))
        nome, chamada = {
            "sha256_hex": ("hashlib.sha256", lambda: sha(args["dados"])),
            "hash_linha": ("cadeia.hash_linha", lambda: self.cadeia.hash_linha(**args)),
            "json_estrito": ("cadeia._carregar", lambda: self.carregar(**args)),
            "json_canonico": ("json.dumps", lambda: canonico(args["valor"])),
            "repr_py": ("repr", lambda: repr(args["valor"])),
            "iguais": ("release.iguais", lambda: self.release.iguais(**args)),
            "hash_fecho": ("release.hash_arvore", lambda: "sha256:" + self.release.hash_arvore(args["arquivos"])),
            "ultima_linha_json": ("release.ultima_linha_json", lambda: self.ultima(**args)),
            "avaliar_saida": ("release.avaliar_saida", lambda: self.release.avaliar_saida(args["saida"], args["criterio"])),
            "controles_aprovados": ("release.controles_aprovados", lambda: self.release.controles_aprovados(
                self.adaptar_execucao(args["execucao"]), args["criterio"])),
            "derivar_registros": ("release.registros", lambda: self.registro(
                args["contrato"], args["execucoes"], args["executados"], args["atuais"],
                args["contrato_sha256"], args["contrato_sha256_depois"])),
            "validar_contrato": ("release.validar_contrato", lambda: self.release.validar_contrato(
                adaptar_contrato(args["contrato"]), "0.0", args["arquivos"])),
            "contrato_gate": ("release.contrato_gate", lambda: self.release.contrato_gate(
                adaptar_contrato(args["contrato"]), args["candidato"])),
            "montar_fecho": ("release.registros + release.contrato_gate + gate43.gate_43 + release.hash_arvore + json.dumps",
                             lambda: self.montar(**args)),
        }[funcao]
        return nome, self.capturar(chamada)

    @staticmethod
    def capturar(chamada):
        try:
            return retorno(chamada())
        except ValueError as erro:
            return {"tipo": "excecao", "classe": "ValueError", "mensagem": str(erro)}


class Casos:
    def __init__(self, oraculo):
        self.oraculo = oraculo
        self.grupos = defaultdict(list)

    def adicionar(self, grupo, funcao, regra, limites, args, *, esperado=None, divergencia=None,
                  gate=None, fecho_sha256=None):
        if gate is None:
            chamada, saida = self.oraculo.chamar(funcao, args)
            oraculo = {"chamada": chamada, "saida": saida}
        else:
            oraculo = {"chamada": "gate43.gate_43", "gate": gate}
        if esperado is None:
            esperado = dict(oraculo["saida"])
            if esperado["tipo"] == "excecao":
                esperado["classe"] = "EntradaInvalida"
        itens = self.grupos[grupo]
        comparacao = "exato" if all(all(" " <= c <= "~" for c in t) for t in textos(etiquetar(args))) else "codigo"
        caso = {
            "id": f"{grupo}{len(itens) + 1:03d}", "grupo": grupo, "funcao": funcao, "args": args,
            "oraculo": oraculo, "esperado": esperado, "comparar": comparacao,
            "divergencia": divergencia, "limite": [f"{grupo}: {l}" for l in limites], "regra": regra,
        }
        if fecho_sha256 is not None:
            caso["fecho_sha256"] = fecho_sha256
        itens.append(deepcopy(caso))
        return esperado["valor"] if esperado["tipo"] == "retorno" else None


def hash_json(casos):
    def caso(funcao, regra, limites, **args):
        return casos.adicionar("H", funcao, regra, limites, args)

    numeros = [1.0, 1.5, 0.1, .0001, 1e-5, 2.5e-5, 1.234e-7, 5e-324, 123456.789,
               1000000000000000.0, 9007199254740991.0, 0.30000000000000004]
    caso("json_canonico", "§2.2", ("vetor §2.2",), valor=numeros)
    caso("json_canonico", "§2.2", ("vetor §2.2",),
         valor=[684191433378107.25, -2112127953860455.25, 213962842088193.625,
                2554252682301.15625, -0.000012345])
    caso("json_canonico", "§2.2", ("vetor §2.2", "ordem maiúscula/é/e+U+0301"),
         valor={"z": .0001, "y": 1e-5, "x": -0.0, "é": "\u2028\x7f\x01", "B": [1, 2.0], "e\u0301": "x"})
    caso("hash_fecho", "§2.6", ("vetor §2.6",), arquivos={"b.py": "0" * 64, "a/x.py": "f" * 64})
    caso("hash_fecho", "§2.6", ("vetor §2.6",),
         arquivos={p: str(i) * 64 for i, p in enumerate(("é.py", "Z.py", "a.py", "a b.py"), 1)})
    linha = '{"anterior":"' + "0" * 64 + '","evento":{"l":[null,true],"n":1.5,"texto":"ação"},"seq":1}'
    caso("hash_linha", "§6.1", ("vetor §6.1",), texto=linha)
    caso("sha256_hex", "§6.1", ("vetor §6.1",), dados=linha.encode())
    for a, b in ((True, 1), (True, 1.0), (1, 1.0)):
        caso("iguais", "§2.3", ("True/1/1.0",), a=a, b=b)
    caso("iguais", "§2.3", ("0.0/-0.0",), a=0.0, b=-0.0)
    caso("iguais", "§2.3", ("ordem maiúscula/é/e+U+0301",), a={"e\u0301": 1}, b={"é": 1})
    exemplos = {
        "2^53-1 int": b"9007199254740991", "2^53-1 float": b"9007199254740991.0",
        "2^53 int": b"9007199254740992", "2^53 float": b"9007199254740992.0",
        "9007199254740991.5": b"9007199254740991.5", "17 dígitos": b"12345678901234567",
        "1e400": b"1e400", "1e-400": b"1e-400", "NaN": b"NaN", "Infinity": b"Infinity",
        "-Infinity": b"-Infinity", "número antes de repetição": b'[{"a":0,"a":1},NaN]',
        "chave a repetida": br'{"a":1,"\u0061":2}',
        "par escapado e direto": '{"\\ud83d\\ude00":1,"😀":2}'.encode(),
        "substituto isolado": br'"\ud800"', "chave substituta repetida": br'{"\ud800":0,"\ud800":1}',
        "profundidade 64": b"[" * 64 + b"0" + b"]" * 64,
        "profundidade 65": b"[" * 65 + b"0" + b"]" * 65,
        "colchetes em texto fechado": b'"' + b"[" * 65 + b'"',
        "colchetes em texto sem fecho": b'"' + b"[" * 65,
        "BOM": b"\xef\xbb\xbf{}", "controle cru": b'"\x01"', "dado extra": b"{} []",
    }
    for limite, dados in exemplos.items():
        caso("json_estrito", "§2.1", (limite,), dados=dados)
    for nome, texto in (("aspas simples", "'"), ("aspas duplas", '"'), ("ambas", "'\""),
                        ("tab", "\t"), ("U+0001", "\x01"), ("U+007F", "\x7f")):
        caso("repr_py", "§2.4", ("repr " + nome,), valor=texto)


def cadeia(casos):
    def caso(funcao, regra, limites, **args):
        return casos.adicionar("K", funcao, regra, limites, args)

    ex = {"texto": "ação", "n": 1.5, "l": [None, True]}
    primeira = caso("proxima_linha", "§6.1/§6.4", ("append idêntico",), dados=b"", evento=ex)
    linha, topo = primeira["linha"], primeira["ancora"]
    segunda = caso("proxima_linha", "§6.1/§6.4", ("append idêntico",),
                   dados=linha, evento={"i": 2}, ancora=topo)
    duas = linha + segunda["linha"]
    for limite, dados, ancora in (
        ("íntegra e ancorada", duas, segunda["ancora"]), ("sem âncora", duas, None),
        ("vazia", b"", None), ("sem LF final", linha[:-1], None),
        ("UTF-8 antes de parse", b"{\n{}\n\xff\n", None),
        ("quebrada com âncora", linha[:-1], topo), ("âncora n bool", linha, dict(topo, n=True)),
        ("âncora cabeça int", linha, dict(topo, cabeca=1)), ("âncora lista", linha, [1]),
        ("âncora n divergente", linha, dict(topo, n=0)),
        ("âncora cabeça divergente", linha, dict(topo, cabeca="0" * 64)),
        ("remoção última linha", linha, segunda["ancora"]),
    ):
        caso("verificar_cadeia", "§6.2", (limite,), dados=dados, ancora=ancora)

    crlf = linha[:-1] + b"\r\n"
    crlf_topo = {"cabeca": sha(crlf[:-1]), "n": 1}
    caso("verificar_cadeia", "§6.2", ("CRLF",), dados=crlf, ancora=crlf_topo)
    objeto = {"anterior": "0" * 64, "evento": {}, "seq": 1}
    for limite, obj in (
        ("chave a mais", dict(objeto, extra=0)),
        ("chave a menos", {"anterior": objeto["anterior"], "seq": 1}),
        ("seq true", dict(objeto, seq=True)), ("seq 1.0", dict(objeto, seq=1.0)),
        ("seq texto", dict(objeto, seq="1")), ("anterior errado", dict(objeto, anterior="x")),
        ("evento lista", dict(objeto, evento=[])),
    ):
        caso("verificar_cadeia", "§6.2", (limite,), dados=canonico(obj) + b"\n")
    caso("proxima_linha", "§6.4", ("append quebrada",), dados=linha[:-1], evento={})
    caso("eventos_cadeia", "§6.3", ("eventos sem âncora",), dados=duas, ancora=None)
    caso("eventos_cadeia", "§6.3", ("íntegra e ancorada",), dados=duas, ancora=segunda["ancora"])


def gate(casos):
    hoje = "2026-01-01"
    reg = {
        "adequado": True, "atual": True, "falha_critica": False, "conflito": False,
        "instrumento": "execucao", "dependencias": {"artefato": "h"},
        "valido_ate": "vinculada_dependencias", "versao_contrato": "1",
    }
    intg = dict(reg, dependencias={"candidato": "id"})
    correntes = {"artefato": "h", "candidato": "id"}
    contrato = {
        "versao": "1", "candidato": "id", "obrigatorios": ["a"],
        "evidencias_aceitas": {"a": ["execucao"]}, "severidade": {"a": "alta"},
        "integracao": {"evidencias_aceitas": ["execucao"]},
    }
    revisao = dict(reg, instrumento="revisao_independente", familia_builder="x", familia_revisor="humano")
    humano = {
        "aprovacao_humana": ["a"], "familias_revisor_aceitas": {"a": ["humano"]},
        "evidencias_aceitas": {"a": ["revisao_independente"]},
    }

    def criterio(limites, *, esperado=None, divergencia=None, **mudancas):
        args = dict(registro=reg, hoje=hoje, aceitos=["execucao"], correntes=correntes)
        args.update(mudancas)
        return casos.adicionar("G", "avaliar_criterio_43", "§3.2", limites, args,
                               esperado=esperado, divergencia=divergencia)

    def completo(limites, *, regra="§3.5", esperado=None, divergencia=None, **mudancas):
        args = dict(contrato=contrato, avaliacoes={"a": reg}, integracao=intg, hoje=hoje, correntes=correntes)
        args.update(mudancas)
        return casos.adicionar("G", "gate_43", regra, limites, args,
                               esperado=esperado, divergencia=divergencia)

    def erro_hoje(valor):
        return excecao(f"R-2.14: hoje deve ser data AAAA-MM-DD, recebido {valor!r}")

    criterio(("E1", "hoje malformado"), hoje="2026-1-01", esperado=erro_hoje("2026-1-01"), divergencia="J-G1")
    criterio(("E2", "correntes lista"), correntes=[])
    criterio(("E3", "exige_humano null"), exige_humano=None)
    criterio(("E4a",), registro=dict(reg, adequado=1, atual=None))
    criterio(("E4b", "instrumento com espaços"), registro=dict(reg, instrumento=" teste "))
    for regra, data in (("E4c", "2026-1-01"), ("E4d", "2026-02-30"), ("E4d", "0000-01-01")):
        criterio((regra, "data " + data), registro=dict(reg, valido_ate=data))

    criterio(("1a",), aceitos=[])
    criterio(("1b",), familias_aceitas=[])
    criterio(("1c",), exige_humano=True)
    completo(("1d", "C1"), regra="§3.2/§3.4", contrato=dict(contrato, versao=None))
    criterio(("2a",), registro=None)
    criterio(("2b",), registro={k: v for k, v in reg.items() if k != "adequado"})
    criterio(("2c",), registro=dict(reg, dependencias={}))
    criterio(("2d", "ordem de inserção R-2.12"), registro=dict(reg, dependencias={"a": None, "z": ""}))
    criterio(("2e", "instrumento vazio"), registro=dict(reg, instrumento=""))
    criterio(("2f",), registro=dict(reg, instrumento="autocritica"), aceitos=["autocritica"])
    criterio(("2g",), aceitos=["teste"])
    criterio(("2h",), registro=dict(revisao, familia_builder=None), aceitos=["revisao_independente"])
    criterio(("2h'", "família não ASCII"), registro=dict(revisao, familia_builder="é"), aceitos=["revisao_independente"],
             divergencia="J-G2", esperado=retorno({
                 "resultado": "UNKNOWN", "motivo": "J-G2: família fora do ASCII; comparação de caixa não é portável",
             }))
    criterio(("2h''", "família x U+001C e X"), registro=dict(revisao, familia_builder="x\x1c", familia_revisor="X"),
             aceitos=["revisao_independente"])
    criterio(("2i",), registro=dict(reg, valido_ate=None))
    for data, limite, itens in (("2025-12-31", "véspera", ("2j",)), ("2026-01-01", "dia", ()),
                               ("2026-01-02", "dia seguinte", ())):
        criterio((*itens, limite), registro=dict(reg, valido_ate=data))
    criterio(("2k",), registro=dict(reg, adequado=False))
    completo(("2l", "versao_contrato ausente"), regra="§3.2",
             avaliacoes={"a": {k: v for k, v in reg.items() if k != "versao_contrato"}})
    for valor, tipo in ((None, "null"), (1, "int"), ([], "lista")):
        completo(("2l" if valor is None else "2l'", "versao_contrato " + tipo), regra="§3.2",
                 avaliacoes={"a": dict(reg, versao_contrato=valor)})
    criterio(("3",), registro=dict(reg, instrumento="medicao"), aceitos=["medicao"])
    criterio(("4a", "duas dependências ausentes"), registro=dict(reg, dependencias={"z": "h", "a": "h"}), correntes={})
    criterio(("4b",), correntes={"artefato": "mudou"})
    criterio(("5a",), familias_aceitas=["humano"])
    criterio(("5b", "5d bloqueado por 5b", "Humano"), familias_aceitas=["humano"], exige_humano=True,
             registro=dict(revisao, familia_revisor="Humano"), aceitos=["revisao_independente"])
    criterio(("5c",), familias_aceitas=["humano"], exige_humano=True, registro=dict(reg, familia_revisor="humano"))
    criterio(("6",))
    criterio(("6",), familias_aceitas=["humano"], exige_humano=True, registro=revisao, aceitos=["revisao_independente"])

    completo(("I1", "C10"), regra="§3.3/§3.4", contrato=dict(contrato, integracao={}))
    completo(("I2", "integração 4.2"), regra="§3.3", integracao={"ok": True, "versao_entregue": "id"})
    completo(("I2", "integração true"), regra="§3.3", integracao=True)
    completo(("I3", "C2"), regra="§3.3/§3.4", contrato=dict(contrato, candidato=None))
    completo(("I4",), regra="§3.3", integracao=dict(intg, dependencias={}))
    completo(("I5",), regra="§3.3", integracao=dict(intg, dependencias={"candidato": "outro"}))
    completo(("I6",), regra="§3.3")
    completo(("I6", "candidato fora de correntes"), regra="§3.3", correntes={"artefato": "h"})

    contratos = (
        ("C3", dict(contrato, obrigatorios=["a", ""])), ("C4", dict(contrato, evidencias_aceitas={})),
        ("C5", dict(contrato, severidade={"a": "urgente"})), ("C6", dict(contrato, familias_revisor_aceitas=None)),
        ("C7", dict(contrato, familias_revisor_aceitas={"a": []})),
        ("C8", dict(contrato, aprovacao_humana=[""])), ("C9", dict(contrato, aprovacao_humana=["b"])),
        ("C9", dict(contrato, aprovacao_humana=["a"])),
        ("C9", {**contrato, **humano, "familias_revisor_aceitas": {}}),
    )
    for codigo, c in contratos:
        limites = (codigo, "obrigatório inválido") if codigo == "C3" else (codigo,)
        completo(limites, regra="§3.4", contrato=c)
    completo(("6",), regra="§3.2/§3.4", contrato={**contrato, **humano}, avaliacoes={"a": revisao})

    multiplos = dict(contrato, obrigatorios=["b", "a"], severidade={"b": "alta", "a": "alta"},
                    evidencias_aceitas={"a": ["execucao"], "b": ["execucao"]})
    precedencia = {"contrato": multiplos, "avaliacoes": {"b": dict(reg, adequado=1), "a": dict(reg, atual=1)},
                   "integracao": dict(intg, conflito=1)}
    completo(("precedência data",), **precedencia, hoje=None, correntes=[],
             esperado=erro_hoje(None), divergencia="J-G1")
    completo(("precedência correntes",), **precedencia, correntes=[])
    completo(("precedência primeiro",), **precedencia)
    precedencia["avaliacoes"]["b"] = reg
    completo(("precedência segundo",), **precedencia)
    precedencia["avaliacoes"]["a"] = reg
    completo(("precedência integração",), **precedencia)

    completo(("registro extra inválido",), avaliacoes={"a": reg, "b": {"adequado": 1}})
    completo(("premissas null",), contrato=dict(contrato, versao=None, premissas=None))
    completo(("premissa candidato",), contrato=dict(contrato, premissas={"a": ["candidato"]}))
    completo(("premissas vazias",), contrato=dict(contrato, premissas={"a": []}))
    completo((), contrato=dict(contrato, premissas={"a": ["ausente"]}))
    completo(("obrigatório repetido",), contrato=dict(contrato, obrigatorios=["a", "a"]))
    completo(("ordem_falhas inválida e empate",),
             contrato=dict(multiplos, obrigatorios=["b", "a", "z"], severidade={"a": "alta", "b": "alta", "z": []}),
             avaliacoes={"a": dict(reg, adequado=False), "b": dict(reg, adequado=False), "z": None})
    completo(("avaliações em pares",), avaliacoes=[["a", reg]], divergencia="J-G3",
             esperado=excecao("R-2.9: avaliacoes deve ser dict ou pares (criterio, registro)"))


def cenario():
    contrato = deepcopy(CONTRATO)
    dados = contrato_bytes(contrato)
    arquivos = {"sonda.py": sha(SONDA.encode()), "valor.txt": sha(b"2\n")}
    execucao = {
        "criterio": "dobro", "comando": ["python", "sonda.py"], "exit_code": 0,
        "stdout": '{"dobro": 4}\n', "stderr": "", "duracao_s": 1.0,
        "controles_negativos": [{"alterado": True, "exit_code": 0, "stdout": '{"dobro": 6}\n', "stderr": ""}],
    }
    return contrato, dados, arquivos, execucao


def derivacao(casos):
    def caso(funcao, regra, limites, *, esperado=None, divergencia=None, **args):
        return casos.adicionar("D", funcao, regra, limites, args, esperado=esperado, divergencia=divergencia)

    oraculo = casos.oraculo
    contrato, dados, arquivos, ex = cenario()
    hc = sha(dados)
    criterio = contrato["criterios"]["dobro"]
    caso("validar_contrato", "§4.1", (), contrato=contrato, arquivos=list(arquivos))
    caso("contrato_gate", "§4.2", (), contrato=contrato, candidato="id")
    mutacao = deepcopy(contrato)
    del mutacao["criterios"]["dobro"]["esperado"]
    mutacao["criterios"]["dobro"]["regra"] = "sem_sobreviventes"
    caso("validar_contrato", "§4.1", ("sem_sobreviventes",), contrato=mutacao, arquivos=list(arquivos),
         esperado=excecao("R-2.1: regra inválida"), divergencia="J-K4")
    caso("avaliar_saida", "§4.4", ("PASS com esperado",), saida={"dobro": 4}, criterio=criterio)
    caso("avaliar_saida", "§4.4", ("todos_aprovados",), saida={"casos": 2, "aprovados": 2, "falhas": []},
         criterio={"regra": "todos_aprovados"})
    caso("avaliar_saida", "§4.4", ("todos_aprovados",), saida={"casos": 0, "aprovados": 0, "falhas": []},
         criterio={"regra": "todos_aprovados"})
    for limite, texto in (
        ("lixo depois da última linha", '{"a":1}\nfim'), ("CRLF", 'log\r\n{"a":1}\r\n'),
        ("U+001C", 'log\x1c{"a":1}\x1cfim'), ("U+00A0 antes de objeto", 'log\n\xa0{"a":1}\nfim'),
        ("última inválida sem recuo", '{"a":1}\n{inválido\nfim'),
    ):
        caso("ultima_linha_json", "§4.3", (limite,), texto=texto)
    for texto in ('{"a":9007199254740992}', '{"a":1e400}',
                  '{"a":' + "[" * 64 + "0" + "]" * 64 + "}"):
        caso("ultima_linha_json", "§4.3", (), texto=texto,
             esperado=retorno(None), divergencia="J-S2")
    for limite, controle in (
        ("controle com stderr", {"stderr": "aviso\n"}),
        ("controle com erro", {"stdout": '{"dobro":6,"erro":"x"}'}),
        ("formato bool e int", {"stdout": '{"dobro":true}'}),
    ):
        resultado = dict(ex["controles_negativos"][0], **controle)
        caso("controles_aprovados", "§4.5", (limite,), criterio=criterio,
             execucao=oraculo.gravada(dict(ex, controles_negativos=[resultado])))

    sem_controles = deepcopy(contrato)
    del sem_controles["criterios"]["dobro"]["controles_negativos"]
    sem_ex = {k: v for k, v in ex.items() if k != "controles_negativos"}
    for limite, c, e in (("PASS com esperado", contrato, ex), ("crítica sem controles", sem_controles, sem_ex)):
        h = sha(contrato_bytes(c))
        registros = oraculo.registro(c, [e], arquivos, arquivos, h, h)
        candidato = registros["correntes"]["candidato"]
        cg = oraculo.release.contrato_gate(adaptar_contrato(c), candidato)
        caso("gate_43", "§4.6/§3.5", (limite,), contrato=cg, hoje=c["data_referencia"], **registros)
    for limite, c, e, a, h in (
        ("PASS com esperado", contrato, ex, arquivos, hc),
        ("crítica sem controles", sem_controles, sem_ex, arquivos, sha(contrato_bytes(sem_controles))),
        ("exit diferente de zero", contrato, dict(ex, exit_code=1), arquivos, hc),
        ("saída None", contrato, dict(ex, stdout="sem JSON\n"), arquivos, hc),
        ("arquivo mudado", contrato, ex, dict(arquivos, **{"valor.txt": sha(b"3\n")}), hc),
        ("arquivo apagado", contrato, ex, {"sonda.py": arquivos["sonda.py"]}, hc),
        ("contrato mudado", contrato, ex, arquivos, "0" * 64),
    ):
        caso("derivar_registros", "§4.6", (limite,), contrato=c, execucoes=[oraculo.gravada(e)],
             executados=arquivos, atuais=a, contrato_sha256=sha(contrato_bytes(c)), contrato_sha256_depois=h)


def fecho(casos):
    contrato, dados, arquivos, ex = cenario()
    args = {
        "contrato_bytes": dados, "caminho_contrato": "jim.json", "execucoes": [ex],
        "arquivos": arquivos, "arquivos_depois": arquivos, "contrato_sha256_depois": sha(dados), "ambiente": {},
    }
    montado = casos.oraculo.montar(**args)
    documento = montado["fecho"]
    gate = documento["gate"]
    if gate["veredito"] != "PASS":
        raise ValueError("o oráculo recusou o cenário sintético do fecho")

    def caso(funcao, regra, limite, args, esperado, f_sha=None, *, divergencia="J-P", referencia=montado):
        casos.adicionar("J", funcao, regra, (limite,), args, esperado=retorno(esperado),
                        divergencia=divergencia, gate=referencia["fecho"]["gate"],
                        fecho_sha256=f_sha or referencia["sha256"])

    caso("montar_fecho", "§5.2", "montar gate PASS", args, montado)
    rep = {"inventario_antes": arquivos, "inventario_depois": arquivos, "execucoes": [ex]}

    def verifica(passo, limite, veredito, motivo, *, fecho_bytes=montado["dados"], atuais=arquivos,
                 contrato_bytes=dados, ancora=None, reproducao=None, referencia=montado, divergencia="J-P"):
        etapa = int(str(passo).removesuffix("b"))
        digest = sha(fecho_bytes)
        args = dict(fecho_bytes=fecho_bytes, contrato_bytes=contrato_bytes, atuais=atuais,
                    reproducao=reproducao, ancora=ancora)
        esperado = {
            "veredito": veredito, "motivo": motivo, "erros": [motivo] if motivo else [], "fecho_sha256": digest,
            "candidato": referencia["fecho"]["candidato"] if etapa >= 3 else None,
            "contrato_sha256": sha(contrato_bytes) if etapa >= 5 else None,
            "gate": referencia["fecho"]["gate"] if etapa >= 9 else None,
            "reproducao": {"resultado": veredito, "motivo": motivo} if etapa == 13 else None,
        }
        caso("verificar_fecho", f"§5.3 V{passo}", limite, args, esperado, digest,
             referencia=referencia, divergencia=divergencia)

    verifica(12, "sem reprodução", "UNPROVEN", "J-P1: PASS do gate sem reprodução externa")
    verifica(13, "reprodução igual", "PASS", None, reproducao=rep)
    verifica(3, "bytes não canônicos", "UNKNOWN", "J-V3: fecho fora da forma canônica",
             fecho_bytes=montado["dados"] + b" ")
    adulterado = canonico(dict(documento, ambiente={"teste": "troca"})) + b"\n"
    verifica("4b", "campo informativo sem novo selo", "FAIL", "J-V4b: selo do fecho não confere",
             fecho_bytes=adulterado)
    verifica(1, "âncora divergente", "FAIL", "J-V1: fecho não confere com a âncora externa", ancora="0" * 64)
    verifica(10, "árvore atual mudou", "FAIL", "R-2.21: árvore atual diverge do fecho: 'valor.txt'",
             atuais=dict(arquivos, **{"valor.txt": sha(b"3\n")}))
    verifica(5, "contrato trocado", "FAIL", "R-2.5: contrato diverge do registrado no fecho",
             contrato_bytes=dados + b" ")
    adulterado = canonico(selar(dict(documento, veredito="PASS"))) + b"\n"
    verifica(9, "PASS gravado", "FAIL", "J-V9: veredito gravado diverge do recalculado", fecho_bytes=adulterado)
    for limite, alteracao in (
        ("stdout adulterado", {"stdout": '{"dobro":6}'}),
        ("saída adulterada", {"saida": {"dobro": 6}}),
    ):
        gravado = dict(documento["execucoes"][0], **alteracao)
        adulterado = canonico(selar(dict(documento, execucoes=[gravado]))) + b"\n"
        verifica(8, limite, "FAIL", "J-N7: stream gravado não confere: dobro", fecho_bytes=adulterado)

    sem_controles = deepcopy(contrato)
    criterio = sem_controles["criterios"]["dobro"]
    del criterio["controles_negativos"]
    criterio["severidade"] = "alta"
    sem_ex = {k: v for k, v in ex.items() if k != "controles_negativos"}
    sem_dados = contrato_bytes(sem_controles)
    base = casos.oraculo.montar(**dict(args, contrato_bytes=sem_dados, execucoes=[sem_ex],
                                      contrato_sha256_depois=sha(sem_dados)))
    adulterado = canonico(selar(dict(base["fecho"], execucoes=documento["execucoes"]))) + b"\n"
    verifica(8, "controles fora do contrato", "FAIL", "J-V8: controles negativos diferem do contrato: dobro",
             fecho_bytes=adulterado, contrato_bytes=sem_dados, reproducao=rep, referencia=base,
             divergencia="J-V8")

    controle = dict(ex["controles_negativos"][0], stderr="aviso\n")
    for passo, limite, muda, veredito, motivo in (
        (7, "reprodução igual", {}, "PASS", None),
        (2, "cópia diferente", {"inventario_antes": {}}, "UNPROVEN", "J-P2: cópia da reprodução não é o fecho"),
        (3, "critério faltando", {"execucoes": []}, "UNPROVEN", "J-P3: reprodução incompleta ou fora de ordem"),
        (4, "exit divergente", {"execucoes": [dict(ex, exit_code=1)]}, "FAIL", "J-P4: reprodução diverge do gravado: dobro"),
        (5, "stderr de controle", {"execucoes": [dict(ex, controles_negativos=[controle])]},
         "FAIL", "J-P5: controle negativo diverge do gravado: dobro#1"),
    ):
        reproducao = dict(rep, **muda)
        args = dict(fecho=documento, contrato_bytes=dados, reproducao=reproducao)
        caso("comparar_reproducao", f"§5.5 P{passo}", limite, args, {"resultado": veredito, "motivo": motivo})
        if muda:
            verifica(13, limite, veredito, motivo, reproducao=reproducao)

    # As duas reproduções ainda rejeitam o artefato alterado. Só P5 distingue as observações.
    oraculo = casos.oraculo
    criterio = contrato["criterios"]["dobro"]
    assert oraculo.release.controles_aprovados(oraculo.adaptar_execucao(ex), criterio)
    for limite, mudanca in (
        ("exit de controle divergente", {"exit_code": 1}),
        ("saída de controle divergente", {"stdout": '{"dobro": 8}\n'}),
    ):
        controle = dict(ex["controles_negativos"][0], **mudanca)
        executada = dict(ex, controles_negativos=[controle])
        assert oraculo.release.controles_aprovados(oraculo.adaptar_execucao(executada), criterio)
        reproduzido = dict(rep, execucoes=[executada])
        motivo = "J-P5: controle negativo diverge do gravado: dobro#1"
        caso("comparar_reproducao", "§5.5 P5", limite,
             dict(fecho=documento, contrato_bytes=dados, reproducao=reproduzido),
             {"resultado": "FAIL", "motivo": motivo})
        verifica(13, limite, "FAIL", motivo, reproducao=reproduzido)


def conferir_git(raiz):
    def git(*args):
        r = subprocess.run(["git", "--no-optional-locks", "-C", raiz, *args],
                           stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=True)
        return r.stdout.strip()

    if not git("rev-parse", "HEAD").startswith(b"f761213") or git("status", "--porcelain", "--", "."):
        raise ValueError("o oráculo deve ser f761213 e estar limpo")


def serializar(valor):
    return json.dumps(etiquetar(valor), ensure_ascii=True, sort_keys=True, indent=1, allow_nan=False) + "\n"


def conferir_privacidade(raiz, conteudo, objetos):
    privados = {raiz, str(Path(raiz).resolve()), os.path.expanduser("~"), gettempdir(),
                os.getcwd(), getpass.getuser(), socket.gethostname()}
    for texto in (*conteudo.values(), *textos(objetos)):
        if any(p and p in texto for p in privados) or CAMINHO_PRIVADO.search(texto):
            raise ValueError("dado local ou identificador privado na saída")


def main():
    raiz = os.environ.get("JIM_ORACULO")
    if not raiz:
        print("Defina JIM_ORACULO com a raiz da v5-alpha.", file=sys.stderr)
        return 2
    try:
        conferir_git(raiz)
        os.environ["PYTHONDONTWRITEBYTECODE"] = "1"
        sys.path[:0] = [raiz, str(Path(raiz) / "tools")]
        from gauntlet_loops import cadeia as cadeia_oraculo, gate43
        import release

        with TemporaryDirectory(prefix="jim-oraculo-") as temporario:
            casos = Casos(Oraculo(Path(temporario), cadeia_oraculo, gate43, release))
            for gerar in (hash_json, cadeia, gate, derivacao, fecho):
                gerar(casos)
        conferir_git(raiz)
        limites = {l for grupo in casos.grupos.values() for caso in grupo for l in caso["limite"]}
        if limites != LIMITES:
            raise ValueError(f"cobertura incorreta ({len(LIMITES - limites)} faltantes, {len(limites - LIMITES)} extras)")
        objetos = {f"{GRUPOS[g]}.json": casos.grupos[g] for g in GRUPOS}
        objetos["ORACULO.json"] = {
            "tag": "v5-alpha", "commit": "f761213", "python": platform.python_version(),
            "unicode": unicodedata.unidata_version, "gerador_sha256": sha(Path(__file__).read_bytes()),
            "limites": sorted(limites),
        }
        conteudo = {nome: serializar(obj) for nome, obj in objetos.items()}
        conferir_privacidade(raiz, conteudo, objetos)
        DIRETORIO.mkdir(exist_ok=True)
        for nome, texto in conteudo.items():
            (DIRETORIO / nome).write_bytes(texto.encode("ascii"))
    except (OSError, ValueError, subprocess.CalledProcessError, ImportError) as erro:
        # Esses erros podem conter caminhos ou dados do ambiente.
        print("Geração recusada. Confira a identidade, a cobertura e a privacidade do oráculo "
              f"({type(erro).__name__}).", file=sys.stderr)
        return 2
    print(" ".join(f"{g}={len(casos.grupos[g])}" for g in GRUPOS))
    return 0


if __name__ == "__main__":
    sys.exit(main())
