"""Observa a execução dos instrumentos e de seus controles."""

import os
import subprocess
import sys
from time import monotonic

from .arquivos import ErroArquivos, caminho_regular, copiar


def executar(copia, comando, timeout_s):
    instrumento = comando[0]
    if instrumento.startswith("-") or caminho_regular(copia, instrumento) is None:
        raise ErroArquivos(f"R-3.13: instrumento indisponível na cópia: {instrumento!r}")
    try:
        processo = subprocess.run(
            [sys.executable, "-B", *comando],
            cwd=copia,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
            timeout=timeout_s,
        )
    except subprocess.TimeoutExpired:
        return {"exit_code": 124, "stdout": "", "stderr": "R-5.3: timeout do instrumento"}
    try:
        return {
            "exit_code": processo.returncode,
            "stdout": processo.stdout.decode("utf-8"),
            "stderr": processo.stderr.decode("utf-8"),
        }
    except UnicodeDecodeError:
        raise ErroArquivos("R-5.16: UTF-8 inválido no stream do instrumento") from None


def executar_controles(copia, inventario, criterio, timeout_s):
    for alteracoes in criterio["controles_negativos"]:
        with copiar(copia, inventario) as (controle, _):
            alterado = False
            for relativo, texto in alteracoes.items():
                arquivo = controle / relativo
                if texto is None:
                    arquivo.unlink()
                    alterado = True
                else:
                    dados = texto.encode("utf-8")
                    if arquivo.read_bytes() != dados:
                        arquivo.write_bytes(dados)
                        alterado = True
            yield {
                "alterado": alterado,
                **executar(controle, criterio["comando"], timeout_s),
            }


def executar_criterios(copia, inventario, contrato):
    execucoes = []
    for nome, criterio in contrato["criterios"].items():
        timeout_s = criterio.get("timeout_s", 900)
        execucao = {"criterio": nome, "comando": ["python", *criterio["comando"]]}
        if "controles_negativos" in criterio:
            execucao["controles_negativos"] = list(
                executar_controles(copia, inventario, criterio, timeout_s)
            )
        inicio = monotonic()
        execucao.update(executar(copia, criterio["comando"], timeout_s))
        execucao["duracao_s"] = monotonic() - inicio
        execucoes.append(execucao)
    return execucoes
