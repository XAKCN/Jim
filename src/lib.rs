//! Jim 0.1.0 — núcleo `jim._core` (ESQUELETO).
//!
//! Interface congelada: contract/jim_core.pyi. Semântica: contract/SEMANTICA.md.
//! Neste esqueleto TODA função levanta `NaoImplementado`: nenhuma devolve
//! resultado e, portanto, nenhuma devolve PASS. A implementação real troca o
//! corpo das funções sem mudar nome, parâmetros, padrões ou exceções.

// A ligação PyO3 fica atrás da feature `python` (padrão). A lógica pura, a
// escrever em módulos irmãos, não usa PyO3 e é testada com
// `cargo test --no-default-features`.
pub mod json;

#[cfg(feature = "python")]
mod ligacao {
    // Esqueleto: os parâmetros só existem para congelar a assinatura Python.
    #![allow(unused_variables)]

    use pyo3::exceptions::{PyNotImplementedError, PyValueError};
    use pyo3::prelude::*;

    pyo3::create_exception!(
        jim._core,
        EntradaInvalida,
        PyValueError,
        "Entrada fora do domínio J-D ou malformada. Nunca acompanha PASS."
    );
    pyo3::create_exception!(
        jim._core,
        NaoImplementado,
        PyNotImplementedError,
        "Função do contrato ainda não implementada. Nunca acompanha PASS."
    );

    const VERSAO: &str = "0.1.0";
    const ESQUEMA_CONTRATO: &str = "jim-contrato/1";
    const ESQUEMA_FECHO: &str = "jim-fecho/1";
    const GENESE: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    fn nao_implementado(nome: &str) -> PyErr {
        NaoImplementado::new_err(format!(
            "J-0: {nome} não implementado no esqueleto {VERSAO}; nenhum resultado (nunca PASS)"
        ))
    }

    /// Argumento cuja omissão é diferente de `None` explícito.
    ///
    /// `exige_humano` omitido vale False; passado como None (ou qualquer não-bool)
    /// é EntradaInvalida R-2.11, como o ValueError da v5-alpha (SEMANTICA §3.2).
    #[allow(dead_code)] // o esqueleto não lê o valor; a implementação lê.
    pub enum Opcional {
        Omitido,
        Valor(Py<PyAny>),
    }

    impl<'a, 'py> FromPyObject<'a, 'py> for Opcional {
        type Error = PyErr;

        fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
            Ok(Opcional::Valor(obj.to_owned().unbind()))
        }
    }

    // ---------------------------------------------------------------- §2 bytes, JSON e hash

    #[pyfunction]
    #[pyo3(text_signature = "(dados)")]
    fn sha256_hex(dados: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("sha256_hex"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(dados)")]
    fn json_estrito(dados: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("json_estrito"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(valor)")]
    fn json_canonico(valor: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("json_canonico"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(a, b)")]
    fn iguais(a: &Bound<'_, PyAny>, b: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("iguais"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(valor)")]
    fn repr_py(valor: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("repr_py"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(caminho)")]
    fn validar_caminho(caminho: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("validar_caminho"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(arquivos)")]
    fn hash_fecho(arquivos: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("hash_fecho"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(raiz, caminhos)")]
    fn inventariar(raiz: &Bound<'_, PyAny>, caminhos: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("inventariar"))
    }

    // ---------------------------------------------------------------- §3 gate

    #[pyfunction]
    #[pyo3(
        signature = (registro, hoje, aceitos, correntes, familias_aceitas=None, exige_humano=Opcional::Omitido),
        text_signature = "(registro, hoje, aceitos, correntes, familias_aceitas=None, exige_humano=False)"
    )]
    fn avaliar_criterio_43(
        registro: &Bound<'_, PyAny>,
        hoje: &Bound<'_, PyAny>,
        aceitos: &Bound<'_, PyAny>,
        correntes: &Bound<'_, PyAny>,
        familias_aceitas: Option<&Bound<'_, PyAny>>,
        exige_humano: Opcional,
    ) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("avaliar_criterio_43"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(contrato, avaliacoes, integracao, hoje, correntes)")]
    fn gate_43(
        contrato: &Bound<'_, PyAny>,
        avaliacoes: &Bound<'_, PyAny>,
        integracao: &Bound<'_, PyAny>,
        hoje: &Bound<'_, PyAny>,
        correntes: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("gate_43"))
    }

    // ---------------------------------------------------------------- §4 derivação de registros

    #[pyfunction]
    #[pyo3(text_signature = "(contrato, arquivos)")]
    fn validar_contrato(contrato: &Bound<'_, PyAny>, arquivos: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("validar_contrato"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(contrato, candidato)")]
    fn contrato_gate(contrato: &Bound<'_, PyAny>, candidato: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("contrato_gate"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(texto)")]
    fn ultima_linha_json(texto: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("ultima_linha_json"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(saida, criterio)")]
    fn avaliar_saida(saida: &Bound<'_, PyAny>, criterio: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("avaliar_saida"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(execucao, criterio)")]
    fn controles_aprovados(execucao: &Bound<'_, PyAny>, criterio: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("controles_aprovados"))
    }

    #[pyfunction]
    #[pyo3(
        text_signature = "(contrato, execucoes, executados, atuais, contrato_sha256, contrato_sha256_depois)"
    )]
    fn derivar_registros(
        contrato: &Bound<'_, PyAny>,
        execucoes: &Bound<'_, PyAny>,
        executados: &Bound<'_, PyAny>,
        atuais: &Bound<'_, PyAny>,
        contrato_sha256: &Bound<'_, PyAny>,
        contrato_sha256_depois: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("derivar_registros"))
    }

    // ---------------------------------------------------------------- §5 fecho

    #[pyfunction]
    #[pyo3(
        text_signature = "(contrato_bytes, caminho_contrato, execucoes, arquivos, arquivos_depois, contrato_sha256_depois, ambiente)"
    )]
    fn montar_fecho(
        contrato_bytes: &Bound<'_, PyAny>,
        caminho_contrato: &Bound<'_, PyAny>,
        execucoes: &Bound<'_, PyAny>,
        arquivos: &Bound<'_, PyAny>,
        arquivos_depois: &Bound<'_, PyAny>,
        contrato_sha256_depois: &Bound<'_, PyAny>,
        ambiente: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("montar_fecho"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(fecho, contrato_bytes, reproducao)")]
    fn comparar_reproducao(
        fecho: &Bound<'_, PyAny>,
        contrato_bytes: &Bound<'_, PyAny>,
        reproducao: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("comparar_reproducao"))
    }

    #[pyfunction]
    #[pyo3(
        signature = (fecho_bytes, contrato_bytes, atuais, reproducao=None, ancora=None),
        text_signature = "(fecho_bytes, contrato_bytes, atuais, reproducao=None, ancora=None)"
    )]
    fn verificar_fecho(
        fecho_bytes: &Bound<'_, PyAny>,
        contrato_bytes: &Bound<'_, PyAny>,
        atuais: &Bound<'_, PyAny>,
        reproducao: Option<&Bound<'_, PyAny>>,
        ancora: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("verificar_fecho"))
    }

    // ---------------------------------------------------------------- §6 cadeia

    #[pyfunction]
    #[pyo3(text_signature = "(texto)")]
    fn hash_linha(texto: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("hash_linha"))
    }

    #[pyfunction]
    #[pyo3(signature = (dados, ancora=None), text_signature = "(dados, ancora=None)")]
    fn verificar_cadeia(dados: &Bound<'_, PyAny>, ancora: Option<&Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("verificar_cadeia"))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(dados, ancora)")]
    fn eventos_cadeia(dados: &Bound<'_, PyAny>, ancora: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("eventos_cadeia"))
    }

    #[pyfunction]
    #[pyo3(signature = (dados, evento, ancora=None), text_signature = "(dados, evento, ancora=None)")]
    fn proxima_linha(
        dados: &Bound<'_, PyAny>,
        evento: &Bound<'_, PyAny>,
        ancora: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        Err(nao_implementado("proxima_linha"))
    }

    // ---------------------------------------------------------------- módulo

    #[pymodule]
    fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
        let py = m.py();
        m.add("VERSAO", VERSAO)?;
        m.add("ESQUEMA_CONTRATO", ESQUEMA_CONTRATO)?;
        m.add("ESQUEMA_FECHO", ESQUEMA_FECHO)?;
        m.add("GENESE", GENESE)?;
        m.add("IMPLEMENTADO", false)?;
        m.add("EntradaInvalida", py.get_type::<EntradaInvalida>())?;
        m.add("NaoImplementado", py.get_type::<NaoImplementado>())?;
        m.add_function(wrap_pyfunction!(sha256_hex, m)?)?;
        m.add_function(wrap_pyfunction!(json_estrito, m)?)?;
        m.add_function(wrap_pyfunction!(json_canonico, m)?)?;
        m.add_function(wrap_pyfunction!(iguais, m)?)?;
        m.add_function(wrap_pyfunction!(repr_py, m)?)?;
        m.add_function(wrap_pyfunction!(validar_caminho, m)?)?;
        m.add_function(wrap_pyfunction!(hash_fecho, m)?)?;
        m.add_function(wrap_pyfunction!(inventariar, m)?)?;
        m.add_function(wrap_pyfunction!(avaliar_criterio_43, m)?)?;
        m.add_function(wrap_pyfunction!(gate_43, m)?)?;
        m.add_function(wrap_pyfunction!(validar_contrato, m)?)?;
        m.add_function(wrap_pyfunction!(contrato_gate, m)?)?;
        m.add_function(wrap_pyfunction!(ultima_linha_json, m)?)?;
        m.add_function(wrap_pyfunction!(avaliar_saida, m)?)?;
        m.add_function(wrap_pyfunction!(controles_aprovados, m)?)?;
        m.add_function(wrap_pyfunction!(derivar_registros, m)?)?;
        m.add_function(wrap_pyfunction!(montar_fecho, m)?)?;
        m.add_function(wrap_pyfunction!(comparar_reproducao, m)?)?;
        m.add_function(wrap_pyfunction!(verificar_fecho, m)?)?;
        m.add_function(wrap_pyfunction!(hash_linha, m)?)?;
        m.add_function(wrap_pyfunction!(verificar_cadeia, m)?)?;
        m.add_function(wrap_pyfunction!(eventos_cadeia, m)?)?;
        m.add_function(wrap_pyfunction!(proxima_linha, m)?)?;
        Ok(())
    }
}
