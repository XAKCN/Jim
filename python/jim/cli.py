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
    def __init__(self, *args, **kwargs):
        super().__init__(*args, add_help=False, **kwargs)
        self.add_argument("-h", "--help", action="help", help="Mostra esta ajuda e sai.")

    def error(self, message):
        raise ErroUso(f"J-CLI: {message}")


def ler_ancora(texto):
    correspondencia = re.fullmatch(r"([0-9]+):([a-f0-9]{64})", texto)
    if correspondencia is None:
        raise argparse.ArgumentTypeError("âncora deve ter o formato N:HEX")
    n, cabeca = correspondencia.groups()
    return {"n": int(n), "cabeca": cabeca}


def parser_cli():
    parser = Parser(prog="jim", description="Registra evidências e verifica provas reproduzíveis.")
    parser.add_argument("--version", action="version", version=f"jim {__version__}",
                        help="Mostra a versão e sai.")
    comandos = parser.add_subparsers(dest="comando", required=True)
    ajuda_provar = "Executa os instrumentos e grava o fecho e a cadeia."
    ajuda_verificar = "Confere o fecho com o contrato e os arquivos atuais."
    provar = comandos.add_parser("provar", help=ajuda_provar, description=ajuda_provar)
    verificar = comandos.add_parser("verificar", help=ajuda_verificar, description=ajuda_verificar)
    for comando in (provar, verificar):
        comando.add_argument("--raiz", type=Path, default=Path("."),
                             help="Diretório do candidato (padrão: diretório atual).")
        comando.add_argument("--contrato", type=Path,
                             help="Caminho do contrato (padrão: RAIZ/jim.json).")
        comando.add_argument("--fecho", type=Path,
                             help="Caminho do fecho (padrão: RAIZ/.jim/fecho.json).")
    provar.add_argument("--cadeia", type=Path,
                        help="Caminho da cadeia (padrão: RAIZ/.jim/cadeia.jsonl).")
    verificar.add_argument("--ancora", metavar="HEX", help="SHA-256 esperado do fecho.")
    verificar.add_argument("--reproduzir", action="store_true",
                           help="Reexecuta os instrumentos de um fecho íntegro que ficou UNPROVEN.")
    ajuda_cadeia = "Opera sobre a cadeia de registros."
    cadeia = comandos.add_parser("cadeia", help=ajuda_cadeia, description=ajuda_cadeia)
    subcomandos = cadeia.add_subparsers(required=True)
    ajuda_conferir = "Confere a integridade da cadeia e a âncora, se fornecida."
    conferir = subcomandos.add_parser("verificar", help=ajuda_conferir, description=ajuda_conferir)
    conferir.add_argument("arquivo", metavar="ARQ", type=Path, help="Arquivo JSONL da cadeia.")
    conferir.add_argument("--ancora", metavar="N:HEX", type=ler_ancora,
                          help="Número e hash esperados da cabeça da cadeia.")
    return parser


def executar(args):
    if args.comando == "cadeia":
        resultado = registro.verificar(args.arquivo, args.ancora)
        if not resultado["integra"] or resultado["ancorada"] is False:
            codigo = 1
        else:
            codigo = 0 if resultado["ancorada"] is True else 3
        return resultado, codigo, resultado["erro"]

    try:
        raiz = args.raiz.resolve(strict=True)
    except (FileNotFoundError, NotADirectoryError):
        raise ErroUso(f"J-CLI: diretório de --raiz não encontrado: {str(args.raiz.absolute())!r}") from None
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
