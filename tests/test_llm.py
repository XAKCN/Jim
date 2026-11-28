from hashlib import sha256
from pathlib import Path
import re
from shutil import copytree, ignore_patterns
from tempfile import TemporaryDirectory
import unittest
from urllib.parse import urlsplit

from jim import _core

from tests.diferencial import vigia
from tests.test_cli import chamar


RAIZ = Path(__file__).resolve().parents[1]
RAW = "https://raw.githubusercontent.com/XAKCN/Jim/main/"
LINK_RAW = re.compile(r'https://raw\.githubusercontent\.com/[^\s)>`\]"|]+')


class DocumentacaoLLMTest(unittest.TestCase):
    def test_entradas_do_gauntlet(self):
        for nome in ("GAUNTLET.md", "llms.txt"):
            with self.subTest(arquivo=nome):
                caminho = RAIZ / nome
                self.assertTrue(caminho.is_file())
                dados = caminho.read_bytes()
                self.assertLess(len(dados), 9_000)
                texto = dados.decode("utf-8")
                for trecho in (
                    "gauntlet armado: modo X (descrição curta); teto Y", "UNPROVEN",
                    "gauntlet armado: modo A (terminal com rede); teto UNPROVEN até reprodução fora do agente",
                    "gauntlet armado: modo B (só web); teto UNPROVEN",
                    "gauntlet armado: modo C (terminal sem rede); teto UNPROVEN",
                    "Modo A", "Modo B", "Modo C", "uvx --from git+https://github.com/XAKCN/Jim",
                ):
                    self.assertIn(trecho, texto)
                if nome == "llms.txt":
                    self.assertEqual(texto.splitlines()[0], "# Jim")

    def test_documentacao_sem_teto_pass(self):
        for nome in ("GAUNTLET.md", "llms.txt", "README.md", "AGENTS.md", "exemplos/minimo/LEIAME.md"):
            with self.subTest(arquivo=nome):
                texto = (RAIZ / nome).read_text(encoding="utf-8")
                self.assertNotIn("teto PASS", texto)

    def test_links_raw(self):
        for nome in ("GAUNTLET.md", "llms.txt", "README.md", "AGENTS.md"):
            texto = (RAIZ / nome).read_text(encoding="utf-8")
            for link in LINK_RAW.findall(texto):
                with self.subTest(arquivo=nome, link=link):
                    self.assertTrue(link.startswith(RAW), link)
                    relativo = urlsplit(link).path.removeprefix("/XAKCN/Jim/main/")
                    destino = (RAIZ / relativo).resolve()
                    self.assertTrue(destino.is_relative_to(RAIZ) and destino.exists(), link)

    def test_readme_aponta_para_llms_txt(self):
        linhas = (RAIZ / "README.md").read_text(encoding="utf-8").splitlines()
        primeira = next(linha for linha in linhas[1:5] if linha.strip())
        self.assertEqual(primeira, f"LLM: leia {RAW}llms.txt")


class ExemploMinimoTest(unittest.TestCase):
    def test_provar_e_verificar(self):
        exemplo = RAIZ / "exemplos/minimo"
        demo = RAIZ / "demo"
        self.assertEqual(_core.json_estrito((exemplo / "jim.json").read_bytes()),
                         _core.json_estrito((demo / "jim.json").read_bytes()))
        self.assertEqual((exemplo / "sonda.py").read_bytes(), (demo / "sonda_honesta.py").read_bytes())
        self.assertEqual((exemplo / "valor.txt").read_bytes(), b"2\n")
        with TemporaryDirectory() as temporario:
            raiz = Path(temporario) / "minimo"
            copytree(exemplo, raiz, ignore=ignore_patterns(".jim", "__pycache__"))
            for comando, opcoes, codigo, veredito in (
                ("provar", (), 3, "UNPROVEN"),
                ("verificar", (), 3, "UNPROVEN"),
                ("verificar", ("--reproduzir",), 0, "PASS"),
            ):
                with self.subTest(comando=comando, opcoes=opcoes):
                    processo = chamar(comando, "--raiz", str(raiz), *opcoes, relogio_fixo=True)
                    self.assertEqual(processo.returncode, codigo, processo)
                    saida = _core.json_estrito(processo.stdout.encode("utf-8"))
                    self.assertEqual(saida["veredito"], veredito)
                    if codigo == 0:
                        digest = sha256((raiz / ".jim/fecho.json").read_bytes()).hexdigest()
                        self.assertEqual(saida["fecho_sha256"], digest)
                        vigia.registrar_fecho(digest)
