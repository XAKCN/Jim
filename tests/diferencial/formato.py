"""Etiquetas dos casos para bytes, floats especiais e tuplas."""

import base64
import math
from pathlib import Path
import json

from .limites import GRUPOS

DIRETORIO = Path(__file__).with_name("casos")


def etiquetar(valor):
    if type(valor) is bytes:
        return {"$bytes": base64.b64encode(valor).decode("ascii")}
    if type(valor) is float and not math.isfinite(valor):
        return {"$float": repr(valor)}
    if type(valor) is tuple:
        return {"$tuple": [etiquetar(v) for v in valor]}
    if type(valor) is list:
        return [etiquetar(v) for v in valor]
    if type(valor) is dict:
        return {k: etiquetar(v) for k, v in valor.items()}
    return valor


def decodificar(valor):
    if type(valor) is list:
        return [decodificar(v) for v in valor]
    if type(valor) is dict:
        if set(valor) == {"$bytes"}:
            return base64.b64decode(valor["$bytes"], validate=True)
        if set(valor) == {"$float"}:
            return float(valor["$float"])
        if set(valor) == {"$tuple"}:
            return tuple(decodificar(v) for v in valor["$tuple"])
        return {k: decodificar(v) for k, v in valor.items()}
    return valor


def carregar_casos():
    for nome in GRUPOS.values():
        yield from json.loads((DIRETORIO / f"{nome}.json").read_text(encoding="ascii"))


def textos(valor):
    if type(valor) is str:
        yield valor
    elif type(valor) in (list, tuple):
        for item in valor:
            yield from textos(item)
    elif type(valor) is dict:
        for chave, item in valor.items():
            yield chave
            yield from textos(item)
    elif type(valor) is bytes:
        # Examina também porções textuais dos bytes etiquetados, mesmo num caso inválido.
        yield valor.decode("utf-8", errors="replace")
