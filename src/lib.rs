//! Jim 0.1.0 — núcleo `jim._core`.
//!
//! Interface congelada: contract/jim_core.pyi. Semântica: contract/SEMANTICA.md.
//! A lógica fica em módulos sem PyO3 (testados com
//! `cargo test --no-default-features`); este arquivo só converte entre Python
//! e o domínio J-D, chama a lógica e mapeia erros para `EntradaInvalida`.

pub mod cadeia;
pub mod fecho;
pub mod gate;
pub mod hash;
pub mod json;
pub mod registros;

pub const VERSAO: &str = "0.1.0";

// A ligação PyO3 fica atrás da feature `python` (padrão).
#[cfg(feature = "python")]
mod ligacao {
    use crate::cadeia::AncoraEntrada;
    use crate::gate::Humano;
    use crate::json::{Objeto, Valor, LIMITE, PROFUNDIDADE_MAXIMA};
    use crate::registros::Inventario;
    use pyo3::exceptions::{PyNotImplementedError, PyValueError};
    use pyo3::prelude::*;
    use pyo3::types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyString};
    use std::panic::{catch_unwind, AssertUnwindSafe};

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

    const ESQUEMA_CONTRATO: &str = crate::registros::ESQUEMA_CONTRATO;
    const ESQUEMA_FECHO: &str = crate::fecho::ESQUEMA_FECHO;
    const GENESE: &str = crate::cadeia::GENESE;

    /// Argumento cuja omissão é diferente de `None` explícito.
    ///
    /// `exige_humano` omitido vale False; passado como None (ou qualquer não-bool)
    /// é EntradaInvalida R-2.11, como o ValueError da v5-alpha (SEMANTICA §3.2).
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

    // ---------------------------------------------------------------- erros e proteção

    fn erro(m: String) -> PyErr {
        EntradaInvalida::new_err(m)
    }

    /// Roda a lógica pura: erro vira EntradaInvalida; pânico vira
    /// EntradaInvalida "J-0: falha interna: …" (G3, §3.6). Nunca PASS.
    fn protegido<T>(f: impl FnOnce() -> Result<T, String>) -> PyResult<T> {
        match catch_unwind(AssertUnwindSafe(f)) {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(m)) => Err(erro(m)),
            Err(p) => {
                let m = p
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| p.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "pânico".into());
                Err(erro(format!("J-0: falha interna: {m}")))
            }
        }
    }

    // ---------------------------------------------------------------- Python → J-D (§1.1)

    fn nome_tipo(o: &Bound<'_, PyAny>) -> String {
        o.get_type()
            .name()
            .and_then(|n| n.to_cow().map(|c| c.into_owned()))
            .unwrap_or_else(|_| "?".into())
    }

    fn prefixo(s: &str, n: usize) -> String {
        s.chars().take(n).collect()
    }

    fn texto_py(o: &Bound<'_, PyAny>) -> Result<String, String> {
        let s = o.cast_exact::<PyString>().map_err(|_| "J-D1: tipo fora do domínio JSON: str".to_string())?;
        s.to_cow()
            .map(|c| c.into_owned())
            .map_err(|_| "R-5.16: texto com substituto UTF-16 isolado".to_string())
    }

    /// Conversão J-D: profundidade, na ordem de inserção, recusa no primeiro problema.
    fn para_valor(o: &Bound<'_, PyAny>, nivel: i64) -> Result<Valor, String> {
        if o.is_none() {
            return Ok(Valor::Nulo);
        }
        if o.is_exact_instance_of::<PyBool>() {
            return o.extract::<bool>().map(Valor::Bool).map_err(|_| "J-D1: tipo fora do domínio JSON: bool".into());
        }
        if o.is_exact_instance_of::<PyInt>() {
            return match o.extract::<i64>() {
                Ok(v) if v.unsigned_abs() <= LIMITE as u64 => Ok(Valor::Int(v)),
                _ => {
                    let s = o.str().and_then(|s| s.to_cow().map(|c| c.into_owned())).unwrap_or_default();
                    Err(format!("R-5.16: inteiro fora de ±(2^53 - 1): {}…", prefixo(&s, 20)))
                }
            };
        }
        if o.is_exact_instance_of::<PyFloat>() {
            let v = o.extract::<f64>().map_err(|_| "J-D1: tipo fora do domínio JSON: float".to_string())?;
            if v.is_finite() && v.abs() <= LIMITE as f64 {
                return Ok(Valor::Float(v));
            }
            let r = o.repr().and_then(|s| s.to_cow().map(|c| c.into_owned())).unwrap_or_default();
            return Err(format!("R-5.16: número fora de ±(2^53 - 1) ou não finito: {}", prefixo(&r, 20)));
        }
        if o.is_exact_instance_of::<PyString>() {
            return texto_py(o).map(Valor::Texto);
        }
        if o.is_exact_instance_of::<PyList>() || o.is_exact_instance_of::<PyDict>() {
            if nivel > PROFUNDIDADE_MAXIMA {
                return Err("R-5.16: mais de 64 níveis de aninhamento".into());
            }
        }
        if let Ok(l) = o.cast_exact::<PyList>() {
            let mut v = Vec::with_capacity(l.len());
            for x in l.iter() {
                v.push(para_valor(&x, nivel + 1)?);
            }
            return Ok(Valor::Lista(v));
        }
        if let Ok(d) = o.cast_exact::<PyDict>() {
            let mut ob = Objeto(Vec::with_capacity(d.len()));
            for (k, x) in d.iter() {
                if !k.is_exact_instance_of::<PyString>() {
                    return Err(format!("J-D1: chave de objeto não é str: {}", nome_tipo(&k)));
                }
                let k = texto_py(&k)?;
                let x = para_valor(&x, nivel + 1)?;
                ob.0.push((k, x));
            }
            return Ok(Valor::Objeto(ob));
        }
        Err(format!("J-D1: tipo fora do domínio JSON: {}", nome_tipo(o)))
    }

    fn json_arg(o: &Bound<'_, PyAny>) -> PyResult<Valor> {
        para_valor(o, 1).map_err(erro)
    }

    // ---------------------------------------------------------------- G2

    fn j_d0(nome: &str, tipo: &str) -> PyErr {
        erro(format!("J-D0: {nome} deve ser {tipo}"))
    }

    fn bytes_arg(o: &Bound<'_, PyAny>, nome: &str) -> PyResult<Vec<u8>> {
        o.cast_exact::<PyBytes>().map(|b| b.as_bytes().to_vec()).map_err(|_| j_d0(nome, "bytes"))
    }

    fn str_arg(o: &Bound<'_, PyAny>, nome: &str) -> PyResult<String> {
        if !o.is_exact_instance_of::<PyString>() {
            return Err(j_d0(nome, "str"));
        }
        texto_py(o).map_err(erro)
    }

    fn lista_str_arg(o: &Bound<'_, PyAny>, nome: &str) -> PyResult<Vec<String>> {
        let l = o.cast_exact::<PyList>().map_err(|_| j_d0(nome, "List[str]"))?;
        let mut v = Vec::with_capacity(l.len());
        for x in l.iter() {
            if !x.is_exact_instance_of::<PyString>() {
                return Err(j_d0(nome, "List[str]"));
            }
            v.push(texto_py(&x).map_err(erro)?);
        }
        Ok(v)
    }

    fn dict_str_arg(o: &Bound<'_, PyAny>, nome: &str) -> PyResult<Inventario> {
        let d = o.cast_exact::<PyDict>().map_err(|_| j_d0(nome, "Dict[str, str]"))?;
        let mut v = Vec::with_capacity(d.len());
        for (k, x) in d.iter() {
            if !k.is_exact_instance_of::<PyString>() || !x.is_exact_instance_of::<PyString>() {
                return Err(j_d0(nome, "Dict[str, str]"));
            }
            v.push((texto_py(&k).map_err(erro)?, texto_py(&x).map_err(erro)?));
        }
        Ok(v)
    }

    /// Âncora da cadeia: nunca levanta (fora de J-D = malformada).
    fn ancora_arg(o: Option<&Bound<'_, PyAny>>) -> AncoraEntrada {
        match o {
            None => AncoraEntrada::Nenhuma,
            Some(a) if a.is_none() => AncoraEntrada::Nenhuma,
            Some(a) => match para_valor(a, 1) {
                Ok(v) => AncoraEntrada::Valor(v),
                Err(_) => AncoraEntrada::Malformada,
            },
        }
    }

    /// Pontos de código de um `str` do Python (substitutos isolados mantidos).
    fn pontos(o: &Bound<'_, PyAny>) -> PyResult<Vec<u32>> {
        let s = o.cast_exact::<PyString>().map_err(|_| j_d0("texto", "str"))?;
        if let Ok(c) = s.to_cow() {
            return Ok(c.chars().map(|c| c as u32).collect());
        }
        let b = s.call_method1("encode", ("utf-8", "surrogatepass"))?;
        let b = b.cast_exact::<PyBytes>().map_err(|_| j_d0("texto", "str"))?.as_bytes().to_vec();
        Ok(decodificar_generalizado(&b))
    }

    /// UTF-8 generalizado (bytes de `surrogatepass`, sempre bem formados).
    fn decodificar_generalizado(b: &[u8]) -> Vec<u32> {
        let mut out = Vec::with_capacity(b.len());
        let mut i = 0;
        while i < b.len() {
            let c = b[i] as u32;
            let (n, ini) = if c < 0x80 {
                (1, c)
            } else if c >> 5 == 0x6 {
                (2, c & 0x1F)
            } else if c >> 4 == 0xE {
                (3, c & 0x0F)
            } else {
                (4, c & 0x07)
            };
            let mut cp = ini;
            for k in 1..n {
                cp = (cp << 6) | (b.get(i + k).copied().unwrap_or(0x80) as u32 & 0x3F);
            }
            out.push(cp);
            i += n;
        }
        out
    }

    // ---------------------------------------------------------------- J-D → Python

    fn para_py<'py>(py: Python<'py>, v: &Valor) -> PyResult<Bound<'py, PyAny>> {
        Ok(match v {
            Valor::Nulo => py.None().into_bound(py),
            Valor::Bool(b) => PyBool::new(py, *b).to_owned().into_any(),
            Valor::Int(i) => i.into_pyobject(py)?.into_any(),
            Valor::Float(f) => PyFloat::new(py, *f).into_any(),
            Valor::Texto(s) => PyString::new(py, s).into_any(),
            Valor::Lista(l) => {
                let lista = PyList::empty(py);
                for x in l {
                    lista.append(para_py(py, x)?)?;
                }
                lista.into_any()
            }
            Valor::Objeto(o) => {
                let d = PyDict::new(py);
                for (k, x) in o.iter() {
                    d.set_item(k, para_py(py, x)?)?;
                }
                d.into_any()
            }
        })
    }

    fn py_de(py: Python<'_>, v: &Valor) -> PyResult<Py<PyAny>> {
        para_py(py, v).map(|b| b.unbind())
    }

    // ---------------------------------------------------------------- §2 bytes, JSON e hash

    #[pyfunction]
    #[pyo3(text_signature = "(dados)")]
    fn sha256_hex(dados: &Bound<'_, PyAny>) -> PyResult<String> {
        let d = bytes_arg(dados, "dados")?;
        protegido(|| Ok(crate::hash::sha256_hex(&d)))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(dados)")]
    fn json_estrito(py: Python<'_>, dados: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let d = bytes_arg(dados, "dados")?;
        let v = protegido(|| crate::json::json_estrito(&d))?;
        py_de(py, &v)
    }

    #[pyfunction]
    #[pyo3(text_signature = "(valor)")]
    fn json_canonico<'py>(py: Python<'py>, valor: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyBytes>> {
        let v = json_arg(valor)?;
        let b = protegido(|| Ok(crate::json::json_canonico(&v)))?;
        Ok(PyBytes::new(py, &b))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(a, b)")]
    fn iguais(a: &Bound<'_, PyAny>, b: &Bound<'_, PyAny>) -> PyResult<bool> {
        let (a, b) = (json_arg(a)?, json_arg(b)?);
        protegido(|| Ok(crate::json::iguais(&a, &b)))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(valor)")]
    fn repr_py(valor: &Bound<'_, PyAny>) -> PyResult<String> {
        let v = json_arg(valor)?;
        protegido(|| Ok(crate::json::repr_py(&v)))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(caminho)")]
    fn validar_caminho(caminho: &Bound<'_, PyAny>) -> PyResult<()> {
        let c = str_arg(caminho, "caminho")?;
        protegido(|| crate::hash::validar_caminho(&c))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(arquivos)")]
    fn hash_fecho(arquivos: &Bound<'_, PyAny>) -> PyResult<String> {
        let a = dict_str_arg(arquivos, "arquivos")?;
        protegido(|| crate::hash::hash_fecho(&a))
    }

    fn inventario_py<'py>(py: Python<'py>, inv: &Inventario) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (k, h) in inv {
            d.set_item(k, h)?;
        }
        Ok(d)
    }

    #[pyfunction]
    #[pyo3(text_signature = "(raiz, caminhos)")]
    fn inventariar<'py>(py: Python<'py>, raiz: &Bound<'py, PyAny>, caminhos: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyDict>> {
        let r = str_arg(raiz, "raiz")?;
        let c = lista_str_arg(caminhos, "caminhos")?;
        let inv = py.detach(|| protegido(|| crate::hash::inventariar(&r, &c)))?;
        inventario_py(py, &inv)
    }

    // ---------------------------------------------------------------- §3 gate

    #[pyfunction]
    #[pyo3(
        signature = (registro, hoje, aceitos, correntes, familias_aceitas=None, exige_humano=Opcional::Omitido),
        text_signature = "(registro, hoje, aceitos, correntes, familias_aceitas=None, exige_humano=False)"
    )]
    fn avaliar_criterio_43(
        py: Python<'_>,
        registro: &Bound<'_, PyAny>,
        hoje: &Bound<'_, PyAny>,
        aceitos: &Bound<'_, PyAny>,
        correntes: &Bound<'_, PyAny>,
        familias_aceitas: Option<&Bound<'_, PyAny>>,
        exige_humano: Opcional,
    ) -> PyResult<Py<PyAny>> {
        let registro = json_arg(registro)?;
        let hoje = json_arg(hoje)?;
        let aceitos = json_arg(aceitos)?;
        let correntes = json_arg(correntes)?;
        let familias = match familias_aceitas {
            Some(f) => json_arg(f)?,
            None => Valor::Nulo,
        };
        let humano = match exige_humano {
            Opcional::Omitido => Humano::Bool(false),
            Opcional::Valor(o) => {
                let o = o.bind(py);
                if o.is_exact_instance_of::<PyBool>() {
                    Humano::Bool(o.extract::<bool>()?)
                } else {
                    let r = o.repr().and_then(|s| s.to_cow().map(|c| c.into_owned())).unwrap_or_else(|_| "?".into());
                    Humano::Invalido(r)
                }
            }
        };
        let p = protegido(|| crate::gate::avaliar_criterio_43(&registro, &hoje, &aceitos, &correntes, &familias, &humano))?;
        py_de(py, &p.para_valor())
    }

    #[pyfunction]
    #[pyo3(text_signature = "(contrato, avaliacoes, integracao, hoje, correntes)")]
    fn gate_43(
        py: Python<'_>,
        contrato: &Bound<'_, PyAny>,
        avaliacoes: &Bound<'_, PyAny>,
        integracao: &Bound<'_, PyAny>,
        hoje: &Bound<'_, PyAny>,
        correntes: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let (c, a, i, h, k) = (json_arg(contrato)?, json_arg(avaliacoes)?, json_arg(integracao)?, json_arg(hoje)?, json_arg(correntes)?);
        let g = protegido(|| crate::gate::gate_43(&c, &a, &i, &h, &k))?;
        py_de(py, &g.para_valor())
    }

    // ---------------------------------------------------------------- §4 derivação de registros

    #[pyfunction]
    #[pyo3(text_signature = "(contrato, arquivos)")]
    fn validar_contrato(contrato: &Bound<'_, PyAny>, arquivos: &Bound<'_, PyAny>) -> PyResult<()> {
        let c = json_arg(contrato)?;
        let a = lista_str_arg(arquivos, "arquivos")?;
        protegido(|| crate::registros::validar_contrato(&c, &a))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(contrato, candidato)")]
    fn contrato_gate(py: Python<'_>, contrato: &Bound<'_, PyAny>, candidato: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let c = json_arg(contrato)?;
        let k = str_arg(candidato, "candidato")?;
        let g = protegido(|| crate::registros::contrato_gate(&c, &k))?;
        py_de(py, &g)
    }

    #[pyfunction]
    #[pyo3(text_signature = "(texto)")]
    fn ultima_linha_json(py: Python<'_>, texto: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        if !texto.is_exact_instance_of::<PyString>() {
            return Err(j_d0("texto", "str"));
        }
        let p = pontos(texto)?;
        let v = protegido(|| Ok(crate::registros::ultima_linha_json_pontos(&p)))?;
        py_de(py, &v.unwrap_or(Valor::Nulo))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(saida, criterio)")]
    fn avaliar_saida(saida: &Bound<'_, PyAny>, criterio: &Bound<'_, PyAny>) -> PyResult<bool> {
        let (s, c) = (json_arg(saida)?, json_arg(criterio)?);
        protegido(|| crate::registros::avaliar_saida(&s, &c))
    }

    #[pyfunction]
    #[pyo3(text_signature = "(execucao, criterio)")]
    fn controles_aprovados(execucao: &Bound<'_, PyAny>, criterio: &Bound<'_, PyAny>) -> PyResult<bool> {
        let (e, c) = (json_arg(execucao)?, json_arg(criterio)?);
        protegido(|| Ok(crate::registros::controles_aprovados(&e, &c)))
    }

    #[pyfunction]
    #[pyo3(
        text_signature = "(contrato, execucoes, executados, atuais, contrato_sha256, contrato_sha256_depois)"
    )]
    fn derivar_registros(
        py: Python<'_>,
        contrato: &Bound<'_, PyAny>,
        execucoes: &Bound<'_, PyAny>,
        executados: &Bound<'_, PyAny>,
        atuais: &Bound<'_, PyAny>,
        contrato_sha256: &Bound<'_, PyAny>,
        contrato_sha256_depois: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let c = json_arg(contrato)?;
        let e = json_arg(execucoes)?;
        let x = dict_str_arg(executados, "executados")?;
        let a = dict_str_arg(atuais, "atuais")?;
        let h = str_arg(contrato_sha256, "contrato_sha256")?;
        let hd = str_arg(contrato_sha256_depois, "contrato_sha256_depois")?;
        let r = protegido(|| crate::registros::derivar_registros(&c, &e, &x, &a, &h, &hd))?;
        py_de(py, &r.para_valor())
    }

    // ---------------------------------------------------------------- §5 fecho

    #[pyfunction]
    #[pyo3(
        text_signature = "(contrato_bytes, caminho_contrato, execucoes, arquivos, arquivos_depois, contrato_sha256_depois, ambiente)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn montar_fecho<'py>(
        py: Python<'py>,
        contrato_bytes: &Bound<'py, PyAny>,
        caminho_contrato: &Bound<'py, PyAny>,
        execucoes: &Bound<'py, PyAny>,
        arquivos: &Bound<'py, PyAny>,
        arquivos_depois: &Bound<'py, PyAny>,
        contrato_sha256_depois: &Bound<'py, PyAny>,
        ambiente: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let cb = bytes_arg(contrato_bytes, "contrato_bytes")?;
        let cc = str_arg(caminho_contrato, "caminho_contrato")?;
        let ex = json_arg(execucoes)?;
        let ar = dict_str_arg(arquivos, "arquivos")?;
        let ad = dict_str_arg(arquivos_depois, "arquivos_depois")?;
        let hd = str_arg(contrato_sha256_depois, "contrato_sha256_depois")?;
        let am = dict_str_arg(ambiente, "ambiente")?;
        let m = protegido(|| crate::fecho::montar_fecho(&cb, &cc, &ex, &ar, &ad, &hd, &am))?;
        if m.veredito == crate::fecho::Veredito::Pass {
            // §5.2: inalcançável; defesa contra PASS na fronteira.
            return Err(erro("J-0: falha interna: fecho com PASS".into()));
        }
        let d = PyDict::new(py);
        d.set_item("fecho", para_py(py, &m.fecho)?)?;
        d.set_item("dados", PyBytes::new(py, &m.dados))?;
        d.set_item("sha256", &m.sha256)?;
        d.set_item("veredito", m.veredito.nome())?;
        Ok(d)
    }

    #[pyfunction]
    #[pyo3(text_signature = "(fecho, contrato_bytes, reproducao)")]
    fn comparar_reproducao(
        py: Python<'_>,
        fecho: &Bound<'_, PyAny>,
        contrato_bytes: &Bound<'_, PyAny>,
        reproducao: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let f = json_arg(fecho)?;
        let cb = bytes_arg(contrato_bytes, "contrato_bytes")?;
        let r = json_arg(reproducao)?;
        let res = protegido(|| crate::fecho::comparar_reproducao(&f, &cb, &r))?;
        py_de(py, &res.para_valor())
    }

    #[pyfunction]
    #[pyo3(
        signature = (fecho_bytes, contrato_bytes, atuais, reproducao=None, ancora=None),
        text_signature = "(fecho_bytes, contrato_bytes, atuais, reproducao=None, ancora=None)"
    )]
    fn verificar_fecho(
        py: Python<'_>,
        fecho_bytes: &Bound<'_, PyAny>,
        contrato_bytes: &Bound<'_, PyAny>,
        atuais: &Bound<'_, PyAny>,
        reproducao: Option<&Bound<'_, PyAny>>,
        ancora: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let fb = bytes_arg(fecho_bytes, "fecho_bytes")?;
        let cb = bytes_arg(contrato_bytes, "contrato_bytes")?;
        let at = dict_str_arg(atuais, "atuais")?;
        let rep = match reproducao {
            Some(r) if !r.is_none() => Some(json_arg(r)?),
            _ => None,
        };
        let an = match ancora {
            Some(a) if !a.is_none() => Some(str_arg(a, "ancora")?),
            _ => None,
        };
        let v = protegido(|| crate::fecho::verificar_fecho(&fb, &cb, &at, rep.as_ref(), an.as_deref()))?;
        py_de(py, &v.para_valor())
    }

    // ---------------------------------------------------------------- §6 cadeia

    #[pyfunction]
    #[pyo3(text_signature = "(texto)")]
    fn hash_linha(texto: &Bound<'_, PyAny>) -> PyResult<String> {
        let t = str_arg(texto, "texto")?;
        protegido(|| Ok(crate::hash::hash_linha(&t)))
    }

    #[pyfunction]
    #[pyo3(signature = (dados, ancora=None), text_signature = "(dados, ancora=None)")]
    fn verificar_cadeia(py: Python<'_>, dados: &Bound<'_, PyAny>, ancora: Option<&Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
        let d = bytes_arg(dados, "dados")?;
        let a = ancora_arg(ancora);
        let v = protegido(|| Ok(crate::cadeia::verificar_cadeia(&d, &a)))?;
        py_de(py, &v.para_valor())
    }

    #[pyfunction]
    #[pyo3(text_signature = "(dados, ancora)")]
    fn eventos_cadeia(py: Python<'_>, dados: &Bound<'_, PyAny>, ancora: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let d = bytes_arg(dados, "dados")?;
        let a = ancora_arg(Some(ancora));
        let ev = protegido(|| crate::cadeia::eventos_cadeia(&d, &a))?;
        py_de(py, &Valor::Lista(ev))
    }

    #[pyfunction]
    #[pyo3(signature = (dados, evento, ancora=None), text_signature = "(dados, evento, ancora=None)")]
    fn proxima_linha<'py>(
        py: Python<'py>,
        dados: &Bound<'py, PyAny>,
        evento: &Bound<'py, PyAny>,
        ancora: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let d = bytes_arg(dados, "dados")?;
        let e = json_arg(evento)?;
        let a = ancora_arg(ancora);
        let (linha, n, cabeca) = protegido(|| crate::cadeia::proxima_linha(&d, &e, &a))?;
        let an = PyDict::new(py);
        an.set_item("n", n)?;
        an.set_item("cabeca", cabeca)?;
        let out = PyDict::new(py);
        out.set_item("linha", PyBytes::new(py, &linha))?;
        out.set_item("ancora", an)?;
        Ok(out)
    }

    // ---------------------------------------------------------------- módulo

    #[pymodule]
    fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
        let py = m.py();
        m.add("VERSAO", crate::VERSAO)?;
        m.add("ESQUEMA_CONTRATO", ESQUEMA_CONTRATO)?;
        m.add("ESQUEMA_FECHO", ESQUEMA_FECHO)?;
        m.add("GENESE", GENESE)?;
        m.add("IMPLEMENTADO", true)?;
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
