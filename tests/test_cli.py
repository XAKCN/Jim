import copy
import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

from jim import _core, cli

from tests.candidato import CONTRATO, preparar


def chamar(*args, raiz=None, relogio_fixo=False):
    if relogio_fixo:
        # A duração é informativa. Fixá-la estabiliza o hash do caso da vigia.
        inicio = [
            "-c",
            "import itertools, sys; from jim import executor, cli; "
            "executor.monotonic = itertools.count(step=1.0).__next__; sys.exit(cli.main())",
        ]
    else:
        inicio = ["-m", "jim.cli"]
    return subprocess.run(
        [sys.executable, "-B", *inicio, *args],
        cwd=raiz,
        env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
        capture_output=True,
        encoding="utf-8",
        timeout=15,
    )


def setUpModule():
    if _core.IMPLEMENTADO:
        from tests.diferencial import vigia

        vigia.instalar()


class CLITest(unittest.TestCase):
    def test_versao(self):
        resultado = chamar("--version")
        self.assertEqual((resultado.returncode, resultado.stdout, resultado.stderr),
                         (0, "jim 0.1.0\n", ""))

    def test_uso(self):
        for args in ((), ("outro",), ("cadeia",), ("cadeia", "verificar", "arq", "--ancora", "x")):
            with self.subTest(args=args):
                resultado = chamar(*args)
                self.assertEqual(resultado.returncode, 4)
                self.assertEqual(resultado.stdout, "")
                self.assertTrue(resultado.stderr.startswith("J-CLI: "), resultado.stderr)

    def test_io(self):
        with TemporaryDirectory() as temporario:
            resultado = chamar("provar", "--raiz", temporario)
        self.assertEqual((resultado.returncode, resultado.stdout), (4, ""))
        self.assertTrue(resultado.stderr.startswith("J-IO: "), resultado.stderr)

    def test_excecoes_do_nucleo(self):
        for excecao in (_core.EntradaInvalida("R-5.16: teste"), _core.NaoImplementado("J-0: teste")):
            with self.subTest(excecao=type(excecao).__name__):
                with patch.object(cli.fluxo, "provar", side_effect=excecao), \
                        patch.object(sys, "stdout") as stdout, patch.object(sys, "stderr") as stderr:
                    self.assertEqual(cli.main(["provar"]), 4)
                self.assertEqual(stdout.mock_calls, [])
                self.assertEqual(stderr.write.call_args_list[0].args, (str(excecao),))

    def test_a7_rejeita_pass_inesperado_do_nucleo(self):
        resultado_invalido = {"veredito": "PASS", "motivo": None, "reproducao": None}
        for args in (["provar"], ["verificar"], ["verificar", "--reproduzir"]):
            with self.subTest(args=args):
                with patch.object(cli.fluxo, args[0], return_value=resultado_invalido), \
                        patch.object(sys, "stdout") as stdout, patch.object(sys, "stderr") as stderr:
                    self.assertEqual(cli.main(args), 4)
                self.assertEqual(stdout.mock_calls, [])
                self.assertTrue(stderr.write.call_args_list[0].args[0].startswith("J-0: "))


@unittest.skipUnless(_core.IMPLEMENTADO, "A7/A8 ponta a ponta dependem de jim._core")
class CLIPontaAPontaTest(unittest.TestCase):
    def setUp(self):
        temporario = TemporaryDirectory()
        self.addCleanup(temporario.cleanup)
        self.raiz = Path(temporario.name)

    def rodar(self, *args, codigo):
        processo = chamar(*args, raiz=self.raiz, relogio_fixo=True)
        saida = json.loads(processo.stdout) if processo.stdout else {}
        if saida.get("veredito") == "PASS" or (processo.returncode == 0 and args[0] in ("provar", "verificar")):
            import hashlib

            from tests.diferencial import vigia

            digest = hashlib.sha256((self.raiz / ".jim" / "fecho.json").read_bytes()).hexdigest()
            self.assertEqual(saida["fecho_sha256"], digest)
            vigia.registrar_fecho(digest)
        self.assertEqual(processo.returncode, codigo, (processo.stdout, processo.stderr))
        self.assertEqual(len(processo.stdout.splitlines()), 1)
        return saida

    def test_a7_a8_candidato_real_e_adulteracoes(self):
        preparar(self.raiz)
        prova = self.rodar("provar", codigo=3)
        self.assertEqual(prova["veredito"], "UNPROVEN")
        self.assertEqual(self.rodar("verificar", codigo=3)["veredito"], "UNPROVEN")
        self.assertEqual(self.rodar("verificar", "--reproduzir", codigo=0)["veredito"], "PASS")
        cadeia = ".jim/cadeia.jsonl"
        self.rodar("cadeia", "verificar", cadeia, codigo=3)
        ancora = prova["ancora_cadeia"]
        ancora_cli = f"{ancora['n']}:{ancora['cabeca']}"
        self.rodar("cadeia", "verificar", cadeia, "--ancora", ancora_cli, codigo=0)
        self.rodar("cadeia", "verificar", cadeia, "--ancora", f"2:{ancora['cabeca']}", codigo=1)

        valor = self.raiz / "valor.txt"
        valor.write_bytes(b"3\n")
        self.rodar("verificar", "--reproduzir", codigo=1)
        valor.write_bytes(b"2\n")
        fecho = self.raiz / ".jim" / "fecho.json"
        dados = fecho.read_bytes()
        sem_selo_novo = dados.replace(b'"duracao_s":1.0', b'"duracao_s":2.0', 1)
        self.assertNotEqual(sem_selo_novo, dados)
        fecho.write_bytes(sem_selo_novo)
        saida = self.rodar("verificar", "--reproduzir", codigo=1)
        self.assertTrue(saida["motivo"].startswith("J-V4b: "), saida)

        corrompido = dados.replace(b'"veredito":"UNPROVEN"', b'"veredito":"XNPROVEN"', 1)
        self.assertNotEqual(corrompido, dados)
        fecho.write_bytes(corrompido)
        saida = self.rodar("verificar", "--reproduzir", codigo=2)
        self.assertEqual(saida["veredito"], "UNKNOWN")

    def test_a8_relogio_quebra_reproducao(self):
        contrato = copy.deepcopy(CONTRATO)
        criterio = contrato["criterios"]["dobro"]
        del criterio["esperado"], criterio["controles_negativos"]
        criterio.update(severidade="alta", regra="todos_aprovados")
        sonda = (
            "import json, time\n"
            "print(json.dumps(dict(casos=1, aprovados=1, falhas=[], instante=str(time.time_ns()))))\n"
        )
        preparar(self.raiz, contrato, sonda)
        self.rodar("provar", codigo=3)
        resultado = self.rodar("verificar", "--reproduzir", codigo=1)
        self.assertTrue(resultado["motivo"].startswith("J-P4: "), resultado)

    def test_reproducao_recusa_diretorio_nao_listavel_no_glob(self):
        if os.geteuid() == 0:
            self.skipTest("root pode listar diretórios sem permissão de leitura")
        preparar(self.raiz, {**CONTRATO, "arquivos": ["**/*.py", "*.txt"]})
        self.rodar("provar", codigo=3)
        oculto = self.raiz / "oculto"
        oculto.mkdir()
        (oculto / "extra.py").touch()
        self.rodar("verificar", "--reproduzir", codigo=1)

        oculto.chmod(0o111)
        try:
            resultado = chamar("verificar", "--reproduzir", raiz=self.raiz)
        finally:
            oculto.chmod(0o700)
        self.assertEqual((resultado.returncode, resultado.stdout), (4, ""), resultado)
        self.assertTrue(resultado.stderr.startswith("J-C7: "), resultado.stderr)
