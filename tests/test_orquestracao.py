import hashlib
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import call, patch

from jim import _core, arquivos, executor, fluxo, registro


def inventariar(raiz, caminhos):
    return {p: hashlib.sha256((Path(raiz) / p).read_bytes()).hexdigest() for p in caminhos}


class OrquestracaoTest(unittest.TestCase):
    def setUp(self):
        temporario = TemporaryDirectory()
        self.addCleanup(temporario.cleanup)
        self.raiz = Path(temporario.name)

    def test_globs_recursivos_ordenados_sem_repeticao(self):
        for p in ("a.py", "d/b.py", ".jim/index.py", "d/__pycache__/x.py", "d/c.txt"):
            nome = self.raiz / p
            nome.parent.mkdir(parents=True, exist_ok=True)
            nome.touch()
        (self.raiz / "atalho.py").symlink_to(self.raiz / "a.py")
        (self.raiz / "atalho").symlink_to(self.raiz / "d", target_is_directory=True)
        with patch.object(_core, "validar_caminho"):
            self.assertEqual(
                arquivos.resolver_arquivos(self.raiz, {"arquivos": ["**", "*.py", "*/b.py"]}),
                ["a.py", "d/b.py", "d/c.txt"],
            )
            with self.assertRaisesRegex(arquivos.PadraoSemArquivos, "^R-3.13:"):
                arquivos.resolver_arquivos(self.raiz, {"arquivos": ["*.py", "d/*.json"]})

    def test_controles_primeiro_e_em_copias_independentes(self):
        (self.raiz / "valor.txt").write_bytes(b"2")
        (self.raiz / "sonda.py").write_text(
            'import os, sys\nfrom pathlib import Path\n'
            'assert os.environ["PYTHONDONTWRITEBYTECODE"] == "1"\n'
            'assert sys.dont_write_bytecode\n'
            'p = Path("valor.txt")\n'
            'sys.stdout.buffer.write((p.read_bytes() if p.exists() else b"ausente") + b"\\r\\n")\n',
            encoding="utf-8",
        )
        itens = ["sonda.py", "valor.txt"]
        contrato = {"criterios": {"c": {"comando": ["sonda.py"], "controles_negativos": [
            {"valor.txt": "3"}, {"valor.txt": None}, {"valor.txt": "2"},
        ]}}}
        with patch.object(_core, "inventariar", side_effect=inventariar), \
                patch.object(executor, "executar", wraps=executor.executar) as executar:
            with arquivos.copiar(self.raiz, inventariar(self.raiz, itens)) as (copia, inventario):
                obtido, = executor.executar_criterios(copia, inventario, contrato)
                self.assertEqual(executar.call_args_list[-1], call(copia, ["sonda.py"], 900))
                self.assertEqual(len({args.args[0] for args in executar.call_args_list}), 4)
                self.assertEqual((copia / "valor.txt").read_bytes(), b"2")
        self.assertEqual(obtido["comando"], ["python", "sonda.py"])
        self.assertEqual(obtido["stdout"], "2\r\n")
        self.assertEqual(obtido["controles_negativos"], [
            {"alterado": True, "exit_code": 0, "stdout": "3\r\n", "stderr": ""},
            {"alterado": True, "exit_code": 0, "stdout": "ausente\r\n", "stderr": ""},
            {"alterado": False, "exit_code": 0, "stdout": "2\r\n", "stderr": ""},
        ])

    def test_timeout_e_utf8_invalido(self):
        sonda = self.raiz / "sonda.py"
        sonda.write_text('import time\nprint("parcial", flush=True)\ntime.sleep(10)\n',
                         encoding="utf-8")
        self.assertEqual(executor.executar(self.raiz, ["sonda.py"], 1), {
            "exit_code": 124, "stdout": "", "stderr": "R-5.3: timeout do instrumento",
        })
        sonda.write_text('import sys\nsys.stderr.buffer.write(b"\\xff")\n', encoding="utf-8")
        with self.assertRaisesRegex(arquivos.ErroArquivos, "^R-5.16:"):
            executor.executar(self.raiz, ["sonda.py"], 1)

    def test_fecho_integro_e_reproducao_explicita_antes_de_executar(self):
        contrato, fecho = self.raiz / "jim.json", self.raiz / "fecho.json"
        contrato.write_bytes(b"contrato")
        fecho.write_bytes(b"fecho")
        for veredito, reproduzir in (("FAIL", True), ("UNKNOWN", True), ("UNPROVEN", False)):
            with self.subTest(veredito=veredito, reproduzir=reproduzir):
                resultado = {"veredito": veredito}
                with patch.object(fluxo, "observar_atuais", return_value=({}, {})), \
                        patch.object(_core, "verificar_fecho", return_value=resultado) as nucleo, \
                        patch.object(fluxo, "executar_criterios") as executar:
                    self.assertEqual(fluxo.verificar(self.raiz, contrato, fecho, None, reproduzir),
                                     resultado)
                executar.assert_not_called()
                nucleo.assert_called_once_with(b"fecho", b"contrato", {}, ancora=None)

    def test_registro_preserva_cadeia_se_nucleo_recusar(self):
        cadeia = self.raiz / "cadeia.jsonl"
        cadeia.write_bytes(b"anterior\n")
        with patch.object(_core, "proxima_linha", side_effect=_core.EntradaInvalida("R-5.16: teste")):
            with self.assertRaises(_core.EntradaInvalida):
                with registro.preparar_evento(cadeia, {"tipo": "teste"}):
                    self.fail("a cadeia recusada liberou a gravação do fecho")
        self.assertEqual(cadeia.read_bytes(), b"anterior\n")

    def test_provar_entrega_observacoes_e_preserva_saida_do_nucleo(self):
        contrato = self.raiz / "jim.json"
        contrato.write_bytes(b"contrato exato\n")
        instrumento = self.raiz / "sonda.py"
        instrumento.write_text('print("observado")\n', encoding="utf-8")
        documento = {"arquivos": ["*.py"], "criterios": {"c": {"comando": ["sonda.py"]}}}
        montado = {
            "fecho": {"candidato": "candidato do núcleo"}, "dados": b"fecho exato\n",
            "sha256": "hash do núcleo", "veredito": "FAIL",
        }
        proxima = {"linha": b"linha exata\n", "ancora": {"n": 1, "cabeca": "ancora do núcleo"}}
        fecho, cadeia = self.raiz / "fecho.json", self.raiz / "cadeia.jsonl"
        with patch.object(_core, "json_estrito", return_value=documento), \
                patch.object(_core, "validar_caminho"), patch.object(_core, "validar_contrato") as validar, \
                patch.object(_core, "inventariar", side_effect=inventariar), \
                patch.object(_core, "sha256_hex", return_value="contrato relido") as hash_contrato, \
                patch.object(_core, "montar_fecho", return_value=montado) as montar, \
                patch.object(_core, "proxima_linha", return_value=proxima) as proxima_linha:
            saida = fluxo.provar(self.raiz, contrato, fecho, cadeia)
        validar.assert_called_once_with(documento, ["sonda.py"])
        hash_contrato.assert_called_once_with(b"contrato exato\n")
        dados, caminho, execucoes, antes, depois, contrato_depois, ambiente = montar.call_args.args
        self.assertEqual((dados, caminho, contrato_depois, ambiente),
                         (b"contrato exato\n", "jim.json", "contrato relido", {}))
        self.assertEqual(antes, inventariar(self.raiz, ["sonda.py"]))
        self.assertEqual(depois, antes)
        self.assertEqual(execucoes[0]["stdout"], "observado\n")
        evento = {"candidato": "candidato do núcleo", "fecho_sha256": "hash do núcleo", "veredito": "FAIL"}
        proxima_linha.assert_called_once_with(b"", {"tipo": "fecho", **evento})
        self.assertEqual(fecho.read_bytes(), b"fecho exato\n")
        self.assertEqual(cadeia.read_bytes(), b"linha exata\n")
        self.assertEqual(saida, {**evento, "ancora_cadeia": proxima["ancora"]})
