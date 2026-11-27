"""Demonstra dois falsos PASS recusados ao vivo pelo Jim instalado."""

import os
from pathlib import Path
from shutil import copyfile
import subprocess
import sys
from tempfile import TemporaryDirectory

from jim import _core


def chamar(raiz, *args):
    return subprocess.run(args, cwd=raiz, capture_output=True, encoding="utf-8",
                          env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"), timeout=15)


def jim(raiz, esperado, *args):
    processo = chamar(raiz, "jim", *args)
    if processo.returncode != esperado:
        detalhe = processo.stderr.strip() or " ".join(args)
        raise SystemExit(f"{detalhe} (exit {processo.returncode}, esperado {esperado})")
    return _core.json_estrito(processo.stdout.encode("utf-8"))


def demonstrar_tautologia(demo, raiz):
    raiz.mkdir()
    copyfile(demo / "jim_tautologia.json", raiz / "jim.json")
    copyfile(demo / "sonda_tautologica.py", raiz / "sonda.py")
    (raiz / "fator.txt").write_bytes(b"2\n")
    (raiz / "fator-quebrado.txt").write_bytes(b"0\n")

    jim(raiz, 1, "provar")
    fecho = _core.json_estrito((raiz / ".jim" / "fecho.json").read_bytes())
    _, teste = fecho["execucoes"]
    controle, = teste["controles_negativos"]
    if (not controle["alterado"] or teste["exit_code"] != 0 or controle["exit_code"] != 0
            or teste["saida"] != {"passou": True} or controle["saida"] != teste["saida"]):
        raise SystemExit("A sonda tautológica deve aprovar as duas implementações.")

    recusado = jim(raiz, 1, "verificar", "--reproduzir")
    criterios = recusado["gate"]["criterios"]
    if (recusado["veredito"] != "FAIL"
            or criterios["teste_dobro"]["resultado"] != "UNKNOWN"
            or criterios["controle_rejeita_codigo_quebrado"]["resultado"] != "FAIL"):
        raise SystemExit("O contrato deve recusar a sonda tautológica.")
    print('FALSO PASS tautológico (exit 0 com código correto e quebrado): {"passou": true}')
    print(f"Jim recusou (FAIL, exit 1): o teste aprovou o código quebrado ({recusado['motivo']})")


def main():
    demo = Path(__file__).resolve().parent
    with TemporaryDirectory(prefix="jim-demo-") as temporario:
        raiz = Path(temporario)
        copyfile(demo / "jim.json", raiz / "jim.json")
        (raiz / "valor.txt").write_bytes(b"2\n")

        copyfile(demo / "sonda_falsa.py", raiz / "sonda.py")
        falso = chamar(raiz, sys.executable, "-B", "sonda.py")
        falso.check_returncode()
        print(f"FALSO PASS alegado pelo instrumento (exit 0): {falso.stdout.strip()}", flush=True)
        jim(raiz, 2, "provar")
        recusado = jim(raiz, 2, "verificar", "--reproduzir")
        print(f"Jim recusou (exit 2): {recusado['motivo']}")

        demonstrar_tautologia(demo, raiz / "tautologia")

        copyfile(demo / "sonda_honesta.py", raiz / "sonda.py")
        prova = jim(raiz, 3, "provar")
        print(f"Candidato honesto, provar (exit 3): {prova['veredito']}")
        verificado = jim(raiz, 0, "verificar", "--reproduzir")
        print(f"Verificar --reproduzir (exit 0): {verificado['veredito']}")


if __name__ == "__main__":
    main()
