"""Vigia de PASS em chamadas ao núcleo e na CLI em subprocesso."""

from functools import wraps
from hashlib import sha256
from inspect import signature
import json

from jim import _core

from .formato import carregar_casos, decodificar, etiquetar

GATES = ("avaliar_criterio_43", "gate_43")
FECHOS = ("comparar_reproducao", "verificar_fecho")
_PASSES = None


def canonico(valor):
    return json.dumps(valor, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode(
        "utf-8", errors="surrogatepass"
    )


def chave(funcao, args):
    if funcao == "verificar_fecho":
        return "fecho", sha256(args["fecho_bytes"]).hexdigest()
    if funcao == "comparar_reproducao":
        return "fecho", sha256(canonico(args["fecho"]) + b"\n").hexdigest()
    return chave_args(funcao, args)


def chave_args(funcao, args):
    return funcao, sha256(canonico(etiquetar(args))).hexdigest()


def passou(resultado):
    return isinstance(resultado, dict) and resultado.get(
        "veredito", resultado.get("resultado")
    ) == "PASS"


def saida_passou(saida):
    return saida["tipo"] == "retorno" and passou(saida["valor"])


def oraculo_passou(caso):
    oraculo = caso["oraculo"]
    return passou(oraculo["gate"]) if "gate" in oraculo else saida_passou(oraculo["saida"])


def pass_indevido(obtido, esperado, local=""):
    """Aponta o primeiro PASS inesperado, inclusive nas partes de um gate."""
    for campo in ("veredito", "resultado"):
        if isinstance(obtido, dict) and obtido.get(campo) == "PASS":
            if not isinstance(esperado, dict) or esperado.get(campo) != "PASS":
                return local + campo
    if isinstance(obtido, dict):
        pares = obtido.items()
    elif isinstance(obtido, list):
        pares = enumerate(obtido)
    else:
        return None
    for campo, valor in pares:
        try:
            previsto = esperado[campo]
        except (KeyError, TypeError, IndexError):
            previsto = None
        caminho = pass_indevido(valor, previsto, f"{local}{campo}.")
        if caminho:
            return caminho
    return None


class Vigia:
    def __init__(self, casos, assinaturas):
        self.registrados = set()
        self.cobertos = set()
        self.conhecidos = set()
        self.vedados = set()
        for caso in casos:
            funcao = caso["funcao"]
            if funcao in assinaturas:
                args = self.argumentos(assinaturas[funcao], **decodificar(caso["args"]))
                k = chave(funcao, args)
                self.conhecidos.add(k)
                if saida_passou(caso["esperado"]) and oraculo_passou(caso):
                    if funcao in FECHOS and caso["fecho_sha256"] != k[1]:
                        raise AssertionError(f"{caso['id']}: fecho_sha256 inconsistente")
                    self.cobertos.add(k)
                else:
                    self.vedados.add(chave_args(funcao, args))

    @staticmethod
    def argumentos(assinatura, *args, **kwargs):
        vinculados = assinatura.bind(*args, **kwargs)
        vinculados.apply_defaults()
        return dict(vinculados.arguments)

    def registrar(self, k):
        self.registrados.add(k)
        if k in self.cobertos:
            return
        funcao, digest = k
        if k in self.conhecidos:
            raise AssertionError(f"PASS INDEVIDO: {funcao} {digest}")
        raise AssertionError(f"ACHADO CANDIDATO: PASS sem caso diferencial: {funcao} {digest}")

    def envolver(self, funcao, original, assinatura):
        @wraps(original)
        def chamar(*args, **kwargs):
            resultado = original(*args, **kwargs)
            if passou(resultado):
                vinculados = self.argumentos(assinatura, *args, **kwargs)
                k = chave(funcao, vinculados)
                if chave_args(funcao, vinculados) in self.vedados:
                    self.registrados.add(k)
                    raise AssertionError(f"PASS INDEVIDO: {k[0]} {k[1]}")
                self.registrar(k)
            return resultado
        return chamar


def instalar():
    global _PASSES
    if _PASSES is not None:
        return _PASSES
    originais = {nome: getattr(_core, nome) for nome in (*GATES, *FECHOS)}
    assinaturas = {nome: signature(funcao) for nome, funcao in originais.items()}
    vigia = Vigia(carregar_casos(), assinaturas)
    for nome, funcao in originais.items():
        setattr(_core, nome, vigia.envolver(nome, funcao, assinaturas[nome]))
    _PASSES = vigia
    return vigia


def registrar_fecho(fecho_sha256):
    instalar().registrar(("fecho", fecho_sha256))
