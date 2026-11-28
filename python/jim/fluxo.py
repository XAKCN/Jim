"""Coordena a preparação, o fecho e a reprodução."""

from . import _core, registro
from .arquivos import (
    ErroArquivos,
    PadraoSemArquivos,
    copiar,
    gravar_atomico,
    inventariar_restantes,
    resolver_arquivos,
)
from .executor import executar_criterios


def ler_contrato(caminho):
    try:
        return caminho.read_bytes()
    except (FileNotFoundError, NotADirectoryError):
        raise ErroArquivos(
            f"J-CLI: contrato não encontrado: {str(caminho)!r} "
            "(crie um jim.json; exemplo pronto em exemplos/minimo/)"
        ) from None


def provar(raiz, contrato, fecho, cadeia):
    dados = ler_contrato(contrato)
    documento = _core.json_estrito(dados)
    caminhos = resolver_arquivos(raiz, documento)
    _core.validar_contrato(documento, caminhos)
    try:
        relativo = contrato.relative_to(raiz).as_posix()
    except ValueError:
        raise ErroArquivos("R-3.13: contrato fora da raiz do candidato") from None
    _core.validar_caminho(relativo)

    saidas = {fecho, cadeia, registro.caminho_trava(cadeia).resolve()}
    entradas = {contrato, *(raiz / p for p in caminhos)}
    if len(saidas) != 3 or saidas & entradas:
        raise ErroArquivos("R-3.13: arquivos de saída conflitam com o candidato ou entre si")

    inventario = _core.inventariar(str(raiz), caminhos)
    with copiar(raiz, inventario) as (copia, executados):
        execucoes = executar_criterios(copia, executados, documento)
        depois = inventariar_restantes(copia, caminhos)
    montado = _core.montar_fecho(
        dados, relativo, execucoes, executados, depois,
        _core.sha256_hex(ler_contrato(contrato)), {},
    )
    evento = {
        "fecho_sha256": montado["sha256"],
        "candidato": montado["fecho"]["candidato"],
        "veredito": montado["veredito"],
    }
    with registro.preparar_evento(cadeia, {"tipo": "fecho", **evento}) as ancora:
        gravar_atomico(fecho, montado["dados"])
    return {**evento, "ancora_cadeia": ancora}


def observar_atuais(raiz, dados):
    try:
        documento = _core.json_estrito(dados)
        caminhos = resolver_arquivos(raiz, documento)
    except _core.EntradaInvalida as erro:
        # O verificador recebe os bytes originais e decide sobre o desacordo.
        if not str(erro).startswith(("R-5.16: ", "J-C1: ", "J-C2: ")):
            raise
        return None, {}
    except PadraoSemArquivos:
        return None, {}
    return documento, _core.inventariar(str(raiz), caminhos)


def verificar(raiz, contrato, fecho, ancora, reproduzir):
    dados_contrato = ler_contrato(contrato)
    dados_fecho = fecho.read_bytes()
    documento, atuais = observar_atuais(raiz, dados_contrato)
    resultado = _core.verificar_fecho(dados_fecho, dados_contrato, atuais, ancora=ancora)
    if not reproduzir or resultado["veredito"] != "UNPROVEN":
        return resultado

    caminhos = _core.json_estrito(dados_fecho)["arquivos"]
    with copiar(raiz, {p: atuais[p] for p in caminhos}) as (copia, antes):
        reproducao = {
            "inventario_antes": antes,
            "execucoes": executar_criterios(copia, antes, documento),
            "inventario_depois": inventariar_restantes(copia, caminhos),
        }
    dados_contrato = ler_contrato(contrato)
    _, atuais = observar_atuais(raiz, dados_contrato)
    return _core.verificar_fecho(dados_fecho, dados_contrato, atuais, reproducao, ancora)
