"""Demonstra um falso PASS recusado ao vivo pelo Jim instalado."""

import json
import os
from pathlib import Path
from shutil import copyfile
import subprocess
import sys
from tempfile import TemporaryDirectory


def chamar(raiz, *args):
    return subprocess.run(args, cwd=raiz, capture_output=True, encoding="utf-8",
                          env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"), timeout=15)


def jim(raiz, esperado, *args):
    processo = chamar(raiz, "jim", *args)
    if processo.returncode != esperado:
        detalhe = processo.stderr.strip() or " ".join(args)
        raise SystemExit(f"{detalhe} (exit {processo.returncode}, esperado {esperado})")
    return json.loads(processo.stdout)


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

        copyfile(demo / "sonda_honesta.py", raiz / "sonda.py")
        prova = jim(raiz, 3, "provar")
        print(f"Candidato honesto, provar (exit 3): {prova['veredito']}")
        verificado = jim(raiz, 0, "verificar", "--reproduzir")
        print(f"Verificar --reproduzir (exit 0): {verificado['veredito']}")


if __name__ == "__main__":
    main()
