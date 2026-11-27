//! §6 de contract/SEMANTICA.md: cadeia de hashes JSONL (semântica de `cadeia`).
//! Sem I/O: quem chama lê o arquivo e grava a linha devolvida.

use crate::hash::sha256_hex;
use crate::json::{carregar, json_canonico, objeto, repr_py, Valor};

pub const GENESE: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Âncora como chegou do Python: ausente, fora de J-D, ou valor J-D.
#[derive(Clone, Debug)]
pub enum AncoraEntrada {
    Nenhuma,
    Malformada,
    Valor(Valor),
}

#[derive(Clone, Debug)]
pub struct VerificacaoCadeia {
    pub integra: bool,
    pub ancorada: Option<bool>,
    pub n: i64,
    pub cabeca: String,
    pub erro: Option<String>,
}

impl VerificacaoCadeia {
    pub fn para_valor(&self) -> Valor {
        objeto(vec![
            ("integra", Valor::Bool(self.integra)),
            ("ancorada", self.ancorada.map(Valor::Bool).unwrap_or(Valor::Nulo)),
            ("n", Valor::Int(self.n)),
            ("cabeca", Valor::texto(&self.cabeca)),
            ("erro", self.erro.as_deref().map(Valor::texto).unwrap_or(Valor::Nulo)),
        ])
    }
}

/// Linhas íntegras: (eventos, n, cabeça), ou o erro.
fn ler(dados: &[u8]) -> Result<(Vec<Valor>, i64, String), String> {
    if dados.is_empty() {
        return Ok((Vec::new(), 0, GENESE.to_string()));
    }
    if dados.last() != Some(&b'\n') {
        return Err("R-5.16: arquivo não termina em \\n".into());
    }
    let corpo = &dados[..dados.len() - 1];
    let mut linhas = Vec::new();
    for b in corpo.split(|c| *c == b'\n') {
        linhas.push(std::str::from_utf8(b).map_err(|_| "R-5.16: UTF-8 inválido".to_string())?);
    }
    let mut anterior = GENESE.to_string();
    let mut eventos = Vec::with_capacity(linhas.len());
    for (i0, texto) in linhas.iter().enumerate() {
        let i = i0 + 1;
        let obj = carregar(texto).map_err(|e| format!("R-5.16: linha {i}: {}", e.strip_prefix("R-5.16: ").unwrap_or(&e)))?;
        let o = match &obj {
            Valor::Objeto(o) if o.len() == 3 && ["anterior", "evento", "seq"].iter().all(|k| o.contem(k)) => o,
            _ => return Err(format!("R-5.16: linha {i}: chaves devem ser exatamente ['anterior', 'evento', 'seq']")),
        };
        let seq = o.bruto("seq").cloned().unwrap_or(Valor::Nulo);
        if !matches!(seq, Valor::Int(s) if s == i as i64) {
            return Err(format!("R-5.16: linha {i}: seq {} fora de ordem", repr_py(&seq)));
        }
        if !matches!(o.bruto("anterior"), Some(v) if v.e_texto(&anterior)) {
            return Err(format!("R-5.16: linha {i}: anterior não confere com o hash da linha {}", i - 1));
        }
        match o.bruto("evento") {
            Some(e @ Valor::Objeto(_)) => eventos.push(e.clone()),
            _ => return Err(format!("R-5.16: linha {i}: evento deve ser objeto")),
        }
        anterior = sha256_hex(texto.as_bytes());
    }
    Ok((eventos, linhas.len() as i64, anterior))
}

/// §6.2 `verificar_cadeia`. Nunca falha.
pub fn verificar_cadeia(dados: &[u8], ancora: &AncoraEntrada) -> VerificacaoCadeia {
    let mut r = VerificacaoCadeia { integra: false, ancorada: None, n: 0, cabeca: GENESE.to_string(), erro: None };
    let (_, n, cabeca) = match ler(dados) {
        Ok(x) => x,
        Err(e) => {
            r.erro = Some(e);
            return r;
        }
    };
    r.integra = true;
    r.n = n;
    r.cabeca = cabeca;
    let malformada = "R-5.16: âncora deve ser {'n': int, 'cabeca': str}";
    match ancora {
        AncoraEntrada::Nenhuma => {}
        AncoraEntrada::Malformada => {
            r.ancorada = Some(false);
            r.erro = Some(malformada.into());
        }
        AncoraEntrada::Valor(a) => {
            let o = a.como_objeto();
            let an = o.and_then(|o| o.bruto("n")).and_then(|v| v.como_int());
            let ac = o.and_then(|o| o.bruto("cabeca")).and_then(|v| v.como_texto());
            match (an, ac) {
                (Some(an), Some(ac)) => {
                    let ok = an == r.n && ac == r.cabeca;
                    r.ancorada = Some(ok);
                    if !ok {
                        r.erro = Some("R-5.16: âncora não confere: linhas removidas ou acrescentadas fora da cadeia".into());
                    }
                }
                _ => {
                    r.ancorada = Some(false);
                    r.erro = Some(malformada.into());
                }
            }
        }
    }
    r
}

/// §6.3 `eventos_cadeia`.
pub fn eventos_cadeia(dados: &[u8], ancora: &AncoraEntrada) -> Result<Vec<Valor>, String> {
    let v = verificar_cadeia(dados, ancora);
    if !(v.integra && v.ancorada == Some(true)) {
        return Err(format!("R-5.16: cadeia não utilizável: {}", v.erro.unwrap_or_else(|| "sem âncora".into())));
    }
    ler(dados).map(|(e, _, _)| e)
}

/// §6.4 `proxima_linha`: (linha com `\n`, n, cabeça).
pub fn proxima_linha(dados: &[u8], evento: &Valor, ancora: &AncoraEntrada) -> Result<(Vec<u8>, i64, String), String> {
    if !matches!(evento, Valor::Objeto(_)) {
        return Err("R-5.16: evento deve ser dict".into());
    }
    let v = verificar_cadeia(dados, ancora);
    if !v.integra {
        return Err(format!("R-5.16: cadeia quebrada, nada foi acrescentado: {}", v.erro.unwrap_or_default()));
    }
    if !matches!(ancora, AncoraEntrada::Nenhuma) && v.ancorada != Some(true) {
        let e = v.erro.map(|e| e.to_string()).unwrap_or_else(|| "None".into());
        return Err(format!("R-5.16: cadeia não confere com a âncora: {e}"));
    }
    let linha = objeto(vec![
        ("seq", Valor::Int(v.n + 1)),
        ("anterior", Valor::texto(&v.cabeca)),
        ("evento", evento.clone()),
    ]);
    let texto = json_canonico(&linha);
    let s = std::str::from_utf8(&texto).map_err(|_| "R-5.16: UTF-8 inválido".to_string())?;
    carregar(s)?;
    let cabeca = sha256_hex(&texto);
    let mut out = texto;
    out.push(b'\n');
    Ok((out, v.n + 1, cabeca))
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::gate::testes::j;

    fn anc(n: i64, c: &str) -> AncoraEntrada {
        AncoraEntrada::Valor(objeto(vec![("n", Valor::Int(n)), ("cabeca", Valor::texto(c))]))
    }

    fn cadeia2() -> (Vec<u8>, String) {
        let (l1, _, _) = proxima_linha(b"", &j(r#"{"texto": "ação", "n": 1.5, "l": [null, true]}"#), &AncoraEntrada::Nenhuma).unwrap();
        let (l2, _, c2) = proxima_linha(&l1, &j(r#"{"i": 2}"#), &AncoraEntrada::Nenhuma).unwrap();
        ([l1, l2].concat(), c2)
    }

    #[test]
    fn vetor_6_1() {
        let (l1, n, c) = proxima_linha(b"", &j(r#"{"texto": "ação", "n": 1.5, "l": [null, true]}"#), &AncoraEntrada::Nenhuma).unwrap();
        assert_eq!(
            String::from_utf8(l1.clone()).unwrap(),
            format!("{{\"anterior\":\"{GENESE}\",\"evento\":{{\"l\":[null,true],\"n\":1.5,\"texto\":\"ação\"}},\"seq\":1}}\n")
        );
        assert_eq!((n, c.as_str()), (1, "6d114df273aba50d44a82f71dbbcb971df099e81feadbc3aa88ab985ebf522e9"));
        let (_, n2, c2) = proxima_linha(&l1, &j(r#"{"i": 2}"#), &anc(1, &c)).unwrap();
        assert_eq!((n2, c2.as_str()), (2, "b03936983a4442a04a2fdd32696f89db38a13d7826133981b01a32f8a3abfd53"));
    }

    #[test]
    fn integra_e_ancora() {
        let (d, c) = cadeia2();
        let v = verificar_cadeia(&d, &anc(2, &c));
        assert!(v.integra && v.ancorada == Some(true) && v.erro.is_none() && v.n == 2);
        let v = verificar_cadeia(&d, &AncoraEntrada::Nenhuma);
        assert!(v.integra && v.ancorada.is_none());
        let v = verificar_cadeia(b"", &AncoraEntrada::Nenhuma);
        assert!(v.integra && v.n == 0 && v.cabeca == GENESE);
        // divergente em n e em cabeça
        for a in [anc(3, &c), anc(2, GENESE)] {
            let v = verificar_cadeia(&d, &a);
            assert_eq!(v.ancorada, Some(false));
            assert_eq!(v.erro.as_deref(), Some("R-5.16: âncora não confere: linhas removidas ou acrescentadas fora da cadeia"));
        }
        // remoção da última linha
        let corte = d.iter().position(|b| *b == b'\n').unwrap() + 1;
        let v = verificar_cadeia(&d[..corte], &anc(2, &c));
        assert!(v.integra && v.ancorada == Some(false));
        // âncora malformada
        for a in [
            AncoraEntrada::Valor(j(r#"{"n": true, "cabeca": "x"}"#)),
            AncoraEntrada::Valor(j(r#"{"n": 1, "cabeca": 5}"#)),
            AncoraEntrada::Valor(j("[1, 2]")),
            AncoraEntrada::Malformada,
        ] {
            let v = verificar_cadeia(&d, &a);
            assert!(v.integra);
            assert_eq!(v.ancorada, Some(false));
            assert_eq!(v.erro.as_deref(), Some("R-5.16: âncora deve ser {'n': int, 'cabeca': str}"));
        }
        // eventos
        let ev = eventos_cadeia(&d, &anc(2, &c)).unwrap();
        assert_eq!(ev.len(), 2);
        assert_eq!(eventos_cadeia(&d, &AncoraEntrada::Nenhuma).unwrap_err(), "R-5.16: cadeia não utilizável: sem âncora");
        assert!(eventos_cadeia(&d, &anc(1, &c)).unwrap_err().starts_with("R-5.16: cadeia não utilizável: R-5.16: âncora não confere"));
    }

    fn erro(d: &[u8]) -> String {
        let v = verificar_cadeia(d, &anc(1, GENESE));
        assert!(!v.integra && v.ancorada.is_none() && v.n == 0 && v.cabeca == GENESE);
        v.erro.unwrap()
    }

    #[test]
    fn quebras() {
        let (d, _) = cadeia2();
        assert_eq!(erro(&d[..d.len() - 1]), "R-5.16: arquivo não termina em \\n");
        // UTF-8 inválido na linha 3 com JSON ruim na linha 1: UTF-8 primeiro
        assert_eq!(erro(b"{ruim\n{}\n\xff\n"), "R-5.16: UTF-8 inválido");
        assert_eq!(erro(b"{ruim\n"), "R-5.16: linha 1: JSON inválido");
        assert_eq!(erro(b"\n"), "R-5.16: linha 1: JSON inválido");
        let g = GENESE;
        let l = |s: String| format!("{s}\n").into_bytes();
        assert_eq!(erro(&l(format!(r#"{{"anterior":"{g}","evento":{{}},"seq":1,"x":1}}"#))), "R-5.16: linha 1: chaves devem ser exatamente ['anterior', 'evento', 'seq']");
        assert_eq!(erro(&l(format!(r#"{{"anterior":"{g}","evento":{{}}}}"#))), "R-5.16: linha 1: chaves devem ser exatamente ['anterior', 'evento', 'seq']");
        for (s, r) in [("true", "True"), ("1.0", "1.0"), ("\"1\"", "'1'"), ("2", "2")] {
            assert_eq!(erro(&l(format!(r#"{{"anterior":"{g}","evento":{{}},"seq":{s}}}"#))), format!("R-5.16: linha 1: seq {r} fora de ordem"));
        }
        assert_eq!(erro(&l(format!(r#"{{"anterior":"{}","evento":{{}},"seq":1}}"#, "1".repeat(64)))), "R-5.16: linha 1: anterior não confere com o hash da linha 0");
        assert_eq!(erro(&l(format!(r#"{{"anterior":"{g}","evento":[],"seq":1}}"#))), "R-5.16: linha 1: evento deve ser objeto");
        assert_eq!(erro(&l(format!(r#"{{"anterior":"{g}","evento":{{"a":1,"a":2}},"seq":1}}"#))), "R-5.16: linha 1: objeto com chave repetida (C4-4)");
        // CRLF: \r é espaço JSON e entra no hash
        let crlf = format!("{{\"anterior\":\"{g}\",\"evento\":{{}},\"seq\":1}}\r\n").into_bytes();
        let v = verificar_cadeia(&crlf, &AncoraEntrada::Nenhuma);
        assert!(v.integra);
        assert_eq!(v.cabeca, sha256_hex(&crlf[..crlf.len() - 1]));
        // edição no meio
        let d2 = String::from_utf8(d.clone()).unwrap().replace("\"i\":2", "\"i\":3").into_bytes();
        assert_ne!(d2, d);
        let v = verificar_cadeia(&d2, &AncoraEntrada::Nenhuma);
        assert!(v.integra); // a última linha muda só a cabeça
        let mut d3 = d.clone();
        let p = d3.iter().position(|b| *b == b'5').unwrap();
        d3[p] = b'6';
        assert_eq!(erro(&d3), "R-5.16: linha 2: anterior não confere com o hash da linha 1");
    }

    #[test]
    fn proxima_recusas() {
        let (d, c) = cadeia2();
        assert_eq!(proxima_linha(&d, &j("[]"), &AncoraEntrada::Nenhuma).unwrap_err(), "R-5.16: evento deve ser dict");
        assert_eq!(proxima_linha(b"x", &j("{}"), &AncoraEntrada::Nenhuma).unwrap_err(), "R-5.16: cadeia quebrada, nada foi acrescentado: R-5.16: arquivo não termina em \\n");
        assert!(proxima_linha(&d, &j("{}"), &anc(1, &c)).unwrap_err().starts_with("R-5.16: cadeia não confere com a âncora: R-5.16: âncora não confere"));
        assert!(proxima_linha(&d, &j("{}"), &AncoraEntrada::Malformada).unwrap_err().starts_with("R-5.16: cadeia não confere com a âncora: R-5.16: âncora deve ser"));
        // profundidade 64 no evento dá 65 na linha
        let fundo = format!("{}{}", "[".repeat(63), "]".repeat(63));
        let ev = j(&format!("{{\"x\": {fundo}}}"));
        assert_eq!(proxima_linha(b"", &ev, &AncoraEntrada::Nenhuma).unwrap_err(), "R-5.16: mais de 64 níveis de aninhamento");
        let ok = j(&format!("{{\"x\": {}{}}}", "[".repeat(62), "]".repeat(62)));
        assert!(proxima_linha(b"", &ok, &AncoraEntrada::Nenhuma).is_ok());
    }
}
