"""Registro local de eventos na cadeia do núcleo."""

import os
from contextlib import contextmanager

from . import _core
from .arquivos import gravar_atomico


def ler(caminho):
    try:
        return caminho.read_bytes()
    except FileNotFoundError:
        return b""


@contextmanager
def bloquear(caminho):
    # O arquivo de lock tem identidade estável durante as trocas atômicas da cadeia.
    if os.name == "nt":
        import msvcrt

        with caminho.open("a+b") as arquivo:
            arquivo.seek(0)
            msvcrt.locking(arquivo.fileno(), msvcrt.LK_LOCK, 1)
            yield
    else:
        import fcntl

        with caminho.open("ab") as arquivo:
            fcntl.flock(arquivo, fcntl.LOCK_EX)
            yield


def caminho_trava(caminho):
    return caminho.with_name(caminho.name + ".lock")


@contextmanager
def preparar_evento(caminho, evento):
    caminho.parent.mkdir(parents=True, exist_ok=True)
    with bloquear(caminho_trava(caminho)):
        dados = ler(caminho)
        proxima = _core.proxima_linha(dados, evento)
        yield proxima["ancora"]
        gravar_atomico(caminho, dados + proxima["linha"])


def verificar(caminho, ancora):
    return _core.verificar_cadeia(ler(caminho), ancora)
