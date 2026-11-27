"""Aplica uma mutação por vez e testa a biblioteca e a roda instalável."""

import argparse
import os
from pathlib import Path
from shutil import copy2, copytree, ignore_patterns
import subprocess
import sys
from tempfile import TemporaryDirectory
from typing import NamedTuple


class Mutante(NamedTuple):
    arquivo: str
    original: str
    alterado: str


MUTANTES = {
    "gate-interno": Mutante("gate.rs", "if INTERNOS.contains(&i) {", "if !INTERNOS.contains(&i) {"),
    "gate-vencimento": Mutante("gate.rs", "if hoje > limite {", "if hoje >= limite {"),
    "gate-dependencia": Mutante(
        "gate.rs", 'if !igual {\n            return Ok(Parte::fail(format!(\n                "R-2.21:',
        'if igual {\n            return Ok(Parte::fail(format!(\n                "R-2.21:',
    ),
    "gate-contrato": Mutante("gate.rs", "if !rc.e_pass() {\n        Res::Unknown",
                            "if !rc.e_pass() {\n        Res::Pass"),
    "gate-integracao": Mutante("gate.rs", "p.e_pass()) && res_int.e_pass()", "p.e_pass())"),
    "fecho-ancora": Mutante("fecho.rs", "if a != v.fecho_sha256 {", "if a == v.fecho_sha256 {"),
    "fecho-canonico": Mutante("fecho.rs", "if canon != fecho_bytes {", "if false && canon != fecho_bytes {"),
    "fecho-selo": Mutante(
        "fecho.rs", "if let Err((ver, m)) = conferir_selo(fo) {",
        "if let Err((ver, m)) = conferir_selo(fo).or(Ok(())) {",
    ),
    "fecho-candidato": Mutante("fecho.rs", "Ok(h) if h == candidato => {}", "Ok(_) => {}"),
    "fecho-controles": Mutante("fecho.rs", '    conferir_controles_do_contrato(criterios, exs).map_err(|e| falha(&format!("J-V8: {e}")))?;\n', ""),
    "fecho-arvore": Mutante("fecho.rs", "if ma.get(p.as_str()) != Some(&h.as_str()) {",
                           "if false && ma.get(p.as_str()) != Some(&h.as_str()) {"),
    "reproducao-ausente": Mutante("fecho.rs", 'None => return parar(v, Veredito::Unproven, "J-P1:',
                                 'None => return parar(v, Veredito::Pass, "J-P1:'),
    "reproducao-copia": Mutante("fecho.rs", "if mapa(antes) != mapa(&conf.arquivos) {",
                              "if false && mapa(antes) != mapa(&conf.arquivos) {"),
    "reproducao-saida": Mutante("fecho.rs", ' || !iguais(&saida_de(r), &saida_g) {', " {"),
    "reproducao-controle-exit": Mutante("fecho.rs", '                    && igual_campo(a, b, "exit_code")\n', ""),
    "reproducao-controle-saida": Mutante("fecho.rs", "                    && iguais(&saida_de(a), &saida_b);", ";"),
    "reproducao-gate": Mutante("fecho.rs", "Ok((_, _, g)) if g.veredito != Res::Pass =>",
                              "Ok((_, _, g)) if false && g.veredito != Res::Pass =>"),
    "registro-exit": Mutante("registros.rs", "let adequado = exit == 0 && avaliar_saida", "let adequado = avaliar_saida"),
    "registro-controle": Mutante("registros.rs", "&& adequado && !controles_aprovados(ex, crit) {",
                                "&& adequado && false {"),
    "registro-hash-stdout": Mutante(
        "registros.rs", 'hash_ok("stdout_sha256", out)\n        && hash_ok("stderr_sha256", err)',
        'hash_ok("stderr_sha256", err)',
    ),
    "registro-hash-stderr": Mutante("registros.rs", '        && hash_ok("stderr_sha256", err)\n', ""),
    "registro-saida": Mutante("registros.rs", "        && iguais(saida, &ultima_linha_json(out).unwrap_or(Valor::Nulo))", ""),
    "hash-arquivo": Mutante("hash.rs", "if !crate::json::e_hex64(h) {", "if false && !crate::json::e_hex64(h) {"),
    "hash-ordem": Mutante("hash.rs", "a.0.as_bytes().cmp(b.0.as_bytes())", "b.0.as_bytes().cmp(a.0.as_bytes())"),
}


def executar(comando, raiz, ambiente, log):
    with log.open("wb") as saida:
        try:
            return subprocess.run(comando, cwd=raiz, env=ambiente, stdout=saida,
                                  stderr=subprocess.STDOUT, timeout=300).returncode
        except subprocess.TimeoutExpired:
            saida.write(b"Tempo limite do comando excedido.\n")
            return 124


def estado(codigo, log, falha_de_teste):
    if codigo == 0:
        return "passou"
    if falha_de_teste in log.read_text(encoding="utf-8", errors="replace"):
        return "falhou"
    return "erro"


def testar(nome, raiz, python, ambiente, logs, rodas):
    pasta = logs / nome
    pasta.mkdir(exist_ok=True)
    cargo = pasta / "cargo.log"
    rust = executar(["cargo", "test", "--locked", "--no-default-features"], raiz, ambiente, cargo)
    rust = estado(rust, cargo, "test result: FAILED.")

    roda = pasta / "roda.log"
    build = executar([python, "-m", "maturin", "build", "--locked", "--profile", "dev",
                      "--interpreter", python, "--out", rodas], raiz, ambiente, roda)
    if build:
        return rust, "erro"
    wheel, = rodas.glob("*.whl")
    instalar = executar([python, "-m", "pip", "install", "--force-reinstall", "--no-deps",
                         "--no-index", wheel], raiz, ambiente, pasta / "instalar.log")
    wheel.unlink()
    if instalar:
        return rust, "erro"

    comparador = pasta / "diferencial.log"
    diferencial = executar([python, "-B", "-m", "unittest", "-v", "tests.diferencial.test_diferencial"],
                           raiz, ambiente, comparador)
    texto = comparador.read_text(encoding="utf-8", errors="replace")
    if "test_contra_o_oraculo (" not in texto or "skipped=" in texto:
        return rust, "erro"
    return rust, estado(diferencial, comparador, "FAILED (")


def campanha(fonte, temporario, logs, nomes):
    copia = temporario / "jim"
    copia.mkdir()
    for caminho in ("Cargo.toml", "Cargo.lock", "pyproject.toml", "README.md", "LICENSE"):
        copy2(fonte / caminho, copia / caminho)
    for pasta in ("src", "python", "tests"):
        copytree(fonte / pasta, copia / pasta, ignore=ignore_patterns("__pycache__"))

    ambiente = {**os.environ, "PYTHONDONTWRITEBYTECODE": "1",
                "CARGO_TARGET_DIR": str(temporario / "target")}
    venv = temporario / "venv"
    subprocess.run([sys.executable, "-m", "venv", venv], check=True, timeout=60)
    python = venv / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
    preparo = executar([python, "-m", "pip", "--isolated", "install", "--index-url",
                        "https://pypi.org/simple", "maturin>=1.9,<2"], copia, ambiente,
                       logs / "preparo.log")
    if preparo:
        raise RuntimeError("falha ao instalar maturin, confira preparo.log")

    rodas = temporario / "rodas"
    resultados = dict.fromkeys(("morto", "vivo", "erro"), 0)
    base = testar("base", copia, python, ambiente, logs, rodas)
    if base != ("passou", "passou"):
        raise RuntimeError(f"a base deve passar nos dois testes (Rust={base[0]}, diferencial={base[1]})")

    with (logs / "resumo.txt").open("w", encoding="utf-8") as resumo:
        def registrar(linha):
            print(linha, flush=True)
            print(linha, file=resumo, flush=True)

        registrar("base Rust=passou diferencial=passou")
        for nome in nomes:
            arquivo, antes, depois = MUTANTES[nome]
            caminho = copia / "src" / arquivo
            original = caminho.read_text(encoding="utf-8")
            if original.count(antes) != 1:
                raise RuntimeError(f"{nome}: a mutação deve corresponder a um único trecho")
            caminho.write_text(original.replace(antes, depois), encoding="utf-8")
            try:
                rust, diferencial = testar(nome, copia, python, ambiente, logs, rodas)
            finally:
                caminho.write_text(original, encoding="utf-8")
            if "erro" in (rust, diferencial):
                resultado = "erro"
            else:
                resultado = "morto" if "falhou" in (rust, diferencial) else "vivo"
            resultados[resultado] += 1
            registrar(f"{nome} {resultado} Rust={rust} diferencial={diferencial}")

        registrar(" ".join(f"{k}={n}" for k, n in resultados.items()))
    return int(bool(resultados["vivo"] or resultados["erro"]))


def main():
    fonte = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mutantes", nargs="*", metavar="MUTANTE", help="omita para executar todos")
    parser.add_argument("--logs", type=Path, default=fonte / ".jim" / "mutacao")
    args = parser.parse_args()
    if desconhecidos := set(args.mutantes) - MUTANTES.keys():
        parser.error("mutantes desconhecidos: " + ", ".join(sorted(desconhecidos)))
    logs = args.logs.resolve()
    logs.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory(prefix="jim-mutacao-") as pasta:
        return campanha(fonte, Path(pasta), logs, args.mutantes or MUTANTES)


if __name__ == "__main__":
    sys.exit(main())
