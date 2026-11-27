"""CLI `jim` (esqueleto da sessão de contrato).

O lado Python substitui este arquivo, mantendo `main(argv=None) -> int`
(CONTRATO.md §4). O esqueleto não emite veredito: só `--version` sai com 0.
"""
import sys

from . import __version__

ERRO = 4  # CONTRATO.md §4: erro de uso, de I/O ou núcleo não implementado.


def main(argv=None):
    args = sys.argv[1:] if argv is None else list(argv)
    if args == ["--version"]:
        print("jim " + __version__)
        return 0
    print("jim " + __version__ + ": CLI não implementada; nenhum veredito emitido", file=sys.stderr)
    return ERRO


if __name__ == "__main__":
    sys.exit(main())
