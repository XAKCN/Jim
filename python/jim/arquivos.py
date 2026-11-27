"""Operações de arquivos da CLI."""

import os
import stat
from contextlib import contextmanager
from pathlib import Path
from tempfile import NamedTemporaryFile, TemporaryDirectory

from . import _core


class ErroArquivos(ValueError):
    """Falha ao preparar os arquivos da prova."""


class PadraoSemArquivos(ErroArquivos):
    """Um padrão do contrato não resolveu para arquivo regular."""


_EXCLUIDOS = {".jim", "__pycache__"}


def caminho_regular(raiz, relativo):
    """Devolve o caminho de um arquivo regular, sem seguir symlinks internos."""
    atual = raiz
    partes = relativo.split("/")
    try:
        for parte in partes[:-1]:
            atual = atual / parte
            if not stat.S_ISDIR(atual.lstat().st_mode):
                return None
        atual = atual / partes[-1]
        if stat.S_ISREG(atual.lstat().st_mode):
            return atual
    except (FileNotFoundError, NotADirectoryError):
        pass
    return None


def resolver_arquivos(raiz, contrato):
    padroes = contrato.get("arquivos") if isinstance(contrato, dict) else None
    if not isinstance(padroes, list) or not padroes or not all(
        isinstance(p, str) and p for p in padroes
    ):
        # O núcleo valida o formato e a precedência dos erros do contrato.
        return []

    caminhos = set()
    for padrao in padroes:
        _core.validar_caminho(padrao)
        achados = set()
        # Antes do CPython 3.13, um ** final em Path.glob só devolve diretórios.
        busca = padrao + "/*" if padrao.split("/")[-1] == "**" else padrao
        for caminho in raiz.glob(busca):
            relativo = caminho.relative_to(raiz)
            if _EXCLUIDOS.isdisjoint(relativo.parts):
                if caminho_regular(raiz, relativo.as_posix()) is not None:
                    achados.add(relativo.as_posix())
        if not achados:
            raise PadraoSemArquivos(f"R-3.13: padrão sem arquivos regulares: {padrao!r}")
        caminhos.update(achados)
    return sorted(caminhos)


@contextmanager
def copiar(raiz, inventario):
    """Materializa os bytes selecionados e confere a cópia antes de usá-la."""
    with TemporaryDirectory(prefix="jim-") as temporario:
        copia = Path(temporario)
        caminhos = list(inventario)
        for relativo in caminhos:
            origem = caminho_regular(raiz, relativo)
            if origem is None:
                raise ErroArquivos(f"R-3.13: arquivo indisponível para cópia: {relativo!r}")
            destino = copia / relativo
            destino.parent.mkdir(parents=True, exist_ok=True)
            destino.write_bytes(origem.read_bytes())
        copiados = _core.inventariar(str(copia), caminhos)
        if copiados != inventario:
            raise ErroArquivos("R-3.13: inventário da cópia diverge da origem")
        yield copia, copiados


def inventariar_restantes(raiz, caminhos):
    restantes = [p for p in caminhos if caminho_regular(raiz, p) is not None]
    return _core.inventariar(str(raiz), restantes)


def gravar_atomico(destino, dados):
    destino.parent.mkdir(parents=True, exist_ok=True)
    with NamedTemporaryFile(dir=destino.parent, delete=False) as arquivo:
        temporario = Path(arquivo.name)
        try:
            arquivo.write(dados)
            arquivo.flush()
            os.fsync(arquivo.fileno())
            arquivo.close()
            os.replace(temporario, destino)
        finally:
            arquivo.close()
            temporario.unlink(missing_ok=True)
