"""Interface de linha de comando do Jim."""

import argparse
import re
import sys
from pathlib import Path

from . import __version__, _core, fluxo, registro
from .arquivos import ErroArquivos

_ERRO = 4
_SEM_REPRODUCAO = {"FAIL": 1, "UNKNOWN": 2, "UNPROVEN": 3}
_COM_REPRODUCAO = {"PASS": 0, **_SEM_REPRODUCAO}


class ErroUso(ValueError):
    pass


class Parser(argparse.ArgumentParser):
    def error(self, message):
        raise ErroUso(f"J-CLI: {message}")


def ler_ancora(texto):
    correspondencia = re.fullmatch(r"([0-9]+):([a-f0-9]{64})", texto)
    if correspondencia is None:
        raise argparse.ArgumentTypeError("âncora deve ter o formato N:HEX")
    n, cabeca = correspondencia.groups()
    return {"n": int(n), "cabeca": cabeca}


def parser_cli():
    parser = Parser(prog="jim")
    parser.add_argument("--version", action="version", version=f"jim {__version__}")
    comandos = parser.add_subparsers(dest="comando", required=True)
    provar = comandos.add_parser("provar")
    verificar = comandos.add_parser("verificar")
    for comando in (provar, verificar):
        comando.add_argument("--raiz", type=Path, default=Path("."))
        comando.add_argument("--contrato", type=Path)
        comando.add_argument("--fecho", type=Path)
    provar.add_argument("--cadeia", type=Path)
    verificar.add_argument("--ancora", metavar="HEX")
    verificar.add_argument("--reproduzir", action="store_true")
    cadeia = comandos.add_parser("cadeia")
    subcomandos = cadeia.add_subparsers(required=True)
    conferir = subcomandos.add_parser("verificar")
    conferir.add_argument("arquivo", metavar="ARQ", type=Path)
    conferir.add_argument("--ancora", metavar="N:HEX", type=ler_ancora)
    return parser


def executar(args):
    if args.comando == "cadeia":
        resultado = registro.verificar(args.arquivo, args.ancora)
        if not resultado["integra"] or resultado["ancorada"] is False:
            codigo = 1
        else:
            codigo = 0 if resultado["ancorada"] is True else 3
        return resultado, codigo, resultado["erro"]

    raiz = args.raiz.resolve(strict=True)
    contrato = (args.contrato or raiz / "jim.json").resolve()
    fecho = (args.fecho or raiz / ".jim" / "fecho.json").resolve()
    if args.comando == "provar":
        cadeia = (args.cadeia or raiz / ".jim" / "cadeia.jsonl").resolve()
        resultado = fluxo.provar(raiz, contrato, fecho, cadeia)
        return resultado, _SEM_REPRODUCAO[resultado["veredito"]], None

    resultado = fluxo.verificar(raiz, contrato, fecho, args.ancora, args.reproduzir)
    codigos = _COM_REPRODUCAO if args.reproduzir and resultado["reproducao"] is not None else _SEM_REPRODUCAO
    return resultado, codigos[resultado["veredito"]], resultado["motivo"]


def diagnosticar(mensagem):
    print(mensagem.split("\n", 1)[0], file=sys.stderr)


def main(argv=None) -> int:
    try:
        args = parser_cli().parse_args(argv)
        resultado, codigo, motivo = executar(args)
        linha = _core.json_canonico(resultado) + b"\n"
        if motivo is not None:
            diagnosticar(motivo)
        sys.stdout.buffer.write(linha)
        return codigo
    except SystemExit as saida:
        return saida.code
    except (ErroUso, ErroArquivos, _core.EntradaInvalida, _core.NaoImplementado) as erro:
        diagnosticar(str(erro))
    except OSError as erro:
        diagnosticar(f"J-IO: erro de sistema ({erro.strerror or type(erro).__name__})")
    except Exception as erro:
        diagnosticar(f"J-0: falha interna ({type(erro).__name__})")
    return _ERRO


if __name__ == "__main__":
    sys.exit(main())
