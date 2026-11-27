//! §1.2 e §2.1–§2.4 de contract/SEMANTICA.md: valor JSON, parse estrito,
//! serialização canônica, igualdade e `repr` do Python.
//!
//! Sem PyO3: testado com `cargo test --no-default-features`.

use std::collections::HashSet;

/// 2^53 − 1 (J-D3, C3-5.1 da v5-alpha).
pub const LIMITE: i64 = (1i64 << 53) - 1;
/// Profundidade máxima de list/dict, contando o próprio valor (J-D5).
pub const PROFUNDIDADE_MAXIMA: i64 = 64;

/// Valor do domínio J-D. Texto nunca tem substituto isolado (Rust `String`).
#[derive(Clone, Debug)]
pub enum Valor {
    Nulo,
    Bool(bool),
    Int(i64),
    Float(f64),
    Texto(String),
    Lista(Vec<Valor>),
    Objeto(Objeto),
}

/// Objeto JSON com ordem de inserção (§1.3). Chaves sem repetição.
#[derive(Clone, Debug, Default)]
pub struct Objeto(pub Vec<(String, Valor)>);

impl Objeto {
    pub fn novo() -> Self {
        Objeto(Vec::new())
    }
    /// Valor da chave, inclusive `null` (presença).
    pub fn bruto(&self, k: &str) -> Option<&Valor> {
        self.0.iter().find(|(c, _)| c == k).map(|(_, v)| v)
    }
    /// `dict.get(k)` do Python: ausente e `null` dão None.
    pub fn get(&self, k: &str) -> Option<&Valor> {
        match self.bruto(k) {
            Some(Valor::Nulo) | None => None,
            v => v,
        }
    }
    pub fn contem(&self, k: &str) -> bool {
        self.0.iter().any(|(c, _)| c == k)
    }
    /// `d[k] = v`: chave existente mantém a posição.
    pub fn inserir(&mut self, k: &str, v: Valor) {
        if let Some(p) = self.0.iter_mut().find(|(c, _)| c == k) {
            p.1 = v;
        } else {
            self.0.push((k.to_string(), v));
        }
    }
    pub fn remover(&mut self, k: &str) {
        self.0.retain(|(c, _)| c != k);
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn chaves(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(c, _)| c.as_str())
    }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Valor)> {
        self.0.iter().map(|(c, v)| (c.as_str(), v))
    }
}

impl Valor {
    pub fn texto(s: &str) -> Valor {
        Valor::Texto(s.to_string())
    }
    pub fn como_texto(&self) -> Option<&str> {
        match self {
            Valor::Texto(s) => Some(s),
            _ => None,
        }
    }
    pub fn como_objeto(&self) -> Option<&Objeto> {
        match self {
            Valor::Objeto(o) => Some(o),
            _ => None,
        }
    }
    pub fn como_lista(&self) -> Option<&Vec<Valor>> {
        match self {
            Valor::Lista(l) => Some(l),
            _ => None,
        }
    }
    /// Inteiro exato (bool não vale: `type(v) is int`).
    pub fn como_int(&self) -> Option<i64> {
        match self {
            Valor::Int(i) => Some(*i),
            _ => None,
        }
    }
    /// `type(v).__name__` do Python.
    pub fn nome_tipo(&self) -> &'static str {
        match self {
            Valor::Nulo => "NoneType",
            Valor::Bool(_) => "bool",
            Valor::Int(_) => "int",
            Valor::Float(_) => "float",
            Valor::Texto(_) => "str",
            Valor::Lista(_) => "list",
            Valor::Objeto(_) => "dict",
        }
    }
    /// Igualdade de texto com um literal (`v == "x"` do Python para v qualquer).
    pub fn e_texto(&self, s: &str) -> bool {
        matches!(self, Valor::Texto(t) if t == s)
    }
}

/// Constrói um objeto a partir de pares (atalho para código e testes).
pub fn objeto(pares: Vec<(&str, Valor)>) -> Valor {
    let mut o = Objeto::novo();
    for (k, v) in pares {
        o.inserir(k, v);
    }
    Valor::Objeto(o)
}

// ---------------------------------------------------------------- §1.2 termos

/// ESPAÇO = `str.isspace()` do Python.
pub fn e_espaco(c: u32) -> bool {
    matches!(c,
        0x09..=0x0D | 0x1C..=0x1F | 0x20 | 0x85 | 0xA0 | 0x1680 | 0x2000..=0x200A
        | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000)
}

/// `str.strip()` do Python.
pub fn strip(s: &str) -> &str {
    s.trim_matches(|c: char| e_espaco(c as u32))
}

/// id válido: `str` com `strip(s) != ""`.
pub fn id_valido(v: Option<&Valor>) -> bool {
    matches!(v, Some(Valor::Texto(s)) if !strip(s).is_empty())
}

/// lista_ids: `list` não vazia de ids válidos.
pub fn lista_ids(v: Option<&Valor>) -> bool {
    match v {
        Some(Valor::Lista(l)) => !l.is_empty() && l.iter().all(|x| id_valido(Some(x))),
        _ => false,
    }
}

/// data AAAA-MM-DD estrita (§1.2), ano 0001–9999.
pub fn e_data(s: &str) -> bool {
    padrao_data(s) && data_valida(s)
}

/// Só o padrão `[0-9]{4}-[0-9]{2}-[0-9]{2}`, inteiro.
pub fn padrao_data(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() })
}

fn data_valida(s: &str) -> bool {
    let n = |a: usize, z: usize| s[a..z].parse::<u32>().unwrap_or(0);
    let (a, m, d) = (n(0, 4), n(5, 7), n(8, 10));
    if a == 0 || !(1..=12).contains(&m) || d == 0 {
        return false;
    }
    let bissexto = (a % 4 == 0 && a % 100 != 0) || a % 400 == 0;
    let dias = [31, if bissexto { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    d <= dias[(m - 1) as usize]
}

/// hex64: `[0-9a-f]{64}`.
pub fn e_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

// ---------------------------------------------------------------- §2.1 json_estrito

/// §2.1 inteiro: decodifica UTF-8 estrito e aplica `carregar`.
pub fn json_estrito(dados: &[u8]) -> Result<Valor, String> {
    let texto = std::str::from_utf8(dados).map_err(|_| "R-5.16: UTF-8 inválido".to_string())?;
    carregar(texto)
}

/// §2.1 passos 2–5 (`cadeia._carregar` da v5-alpha) sobre texto já decodificado.
pub fn carregar(texto: &str) -> Result<Valor, String> {
    if profundidade(texto) > PROFUNDIDADE_MAXIMA {
        return Err("R-5.16: mais de 64 níveis de aninhamento".into());
    }
    let mut p = Parser { b: texto.as_bytes(), i: 0, repetida: false, substituto: false, nivel: 0 };
    p.espaco();
    let v = p.valor()?;
    p.espaco();
    if p.i != p.b.len() {
        return Err(invalido());
    }
    if p.repetida {
        return Err("R-5.16: objeto com chave repetida (C4-4)".into());
    }
    if p.substituto {
        return Err("R-5.16: texto com substituto UTF-16 isolado".into());
    }
    Ok(v)
}

fn invalido() -> String {
    "R-5.16: JSON inválido".into()
}

/// §2.1 passo 2: maior aninhamento fora de literais de texto.
pub fn profundidade(texto: &str) -> i64 {
    let b = texto.as_bytes();
    let (mut atual, mut maior) = (0i64, 0i64);
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'"' {
            // tenta casar "(?:[^"\\]|\\.)*"
            let mut j = i + 1;
            let fim = loop {
                if j >= b.len() {
                    break None;
                }
                match b[j] {
                    b'"' => break Some(j),
                    b'\\' => {
                        if j + 1 < b.len() {
                            j += 2;
                        } else {
                            break None;
                        }
                    }
                    _ => j += 1,
                }
            };
            if let Some(f) = fim {
                i = f + 1;
                continue;
            }
        } else if c == b'[' || c == b'{' {
            atual += 1;
            if atual > maior {
                maior = atual;
                if maior > PROFUNDIDADE_MAXIMA {
                    return maior;
                }
            }
        } else if c == b']' || c == b'}' {
            atual -= 1;
        }
        i += 1;
    }
    maior
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    repetida: bool,
    substituto: bool,
    nivel: i64,
}

impl Parser<'_> {
    fn espaco(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn resto(&self, s: &str) -> bool {
        self.b[self.i..].starts_with(s.as_bytes())
    }

    fn valor(&mut self) -> Result<Valor, String> {
        if self.i >= self.b.len() {
            return Err(invalido());
        }
        match self.b[self.i] {
            b'"' => {
                let (s, _) = self.texto()?;
                Ok(Valor::Texto(s))
            }
            b'{' => self.objeto(),
            b'[' => self.lista(),
            b'n' if self.resto("null") => {
                self.i += 4;
                Ok(Valor::Nulo)
            }
            b't' if self.resto("true") => {
                self.i += 4;
                Ok(Valor::Bool(true))
            }
            b'f' if self.resto("false") => {
                self.i += 5;
                Ok(Valor::Bool(false))
            }
            b'N' if self.resto("NaN") => Err("R-5.16: NaN não é número JSON finito".into()),
            b'I' if self.resto("Infinity") => Err("R-5.16: Infinity não é número JSON finito".into()),
            b'-' if self.resto("-Infinity") => Err("R-5.16: -Infinity não é número JSON finito".into()),
            b'-' | b'0'..=b'9' => self.numero(),
            _ => Err(invalido()),
        }
    }

    fn entrar(&mut self) -> Result<(), String> {
        self.nivel += 1;
        // Salvaguarda: o passo 2 já limita a 64; isto só protege a pilha.
        if self.nivel > PROFUNDIDADE_MAXIMA + 1 {
            return Err("R-5.16: mais de 64 níveis de aninhamento".into());
        }
        Ok(())
    }

    fn objeto(&mut self) -> Result<Valor, String> {
        self.entrar()?;
        self.i += 1;
        let mut o = Objeto::novo();
        let mut vistas: HashSet<Vec<u8>> = HashSet::new();
        self.espaco();
        if self.i < self.b.len() && self.b[self.i] == b'}' {
            self.i += 1;
            self.nivel -= 1;
            return Ok(Valor::Objeto(o));
        }
        loop {
            if self.i >= self.b.len() || self.b[self.i] != b'"' {
                return Err(invalido());
            }
            let (k, bruta) = self.texto()?;
            self.espaco();
            if self.i >= self.b.len() || self.b[self.i] != b':' {
                return Err(invalido());
            }
            self.i += 1;
            self.espaco();
            let v = self.valor()?;
            if !vistas.insert(bruta) {
                self.repetida = true;
            } else {
                o.0.push((k, v));
            }
            self.espaco();
            if self.i >= self.b.len() {
                return Err(invalido());
            }
            match self.b[self.i] {
                b',' => {
                    self.i += 1;
                    self.espaco();
                }
                b'}' => {
                    self.i += 1;
                    self.nivel -= 1;
                    return Ok(Valor::Objeto(o));
                }
                _ => return Err(invalido()),
            }
        }
    }

    fn lista(&mut self) -> Result<Valor, String> {
        self.entrar()?;
        self.i += 1;
        let mut l = Vec::new();
        self.espaco();
        if self.i < self.b.len() && self.b[self.i] == b']' {
            self.i += 1;
            self.nivel -= 1;
            return Ok(Valor::Lista(l));
        }
        loop {
            l.push(self.valor()?);
            self.espaco();
            if self.i >= self.b.len() {
                return Err(invalido());
            }
            match self.b[self.i] {
                b',' => {
                    self.i += 1;
                    self.espaco();
                }
                b']' => {
                    self.i += 1;
                    self.nivel -= 1;
                    return Ok(Valor::Lista(l));
                }
                _ => return Err(invalido()),
            }
        }
    }

    fn hex4(&self, a: usize) -> Option<u32> {
        if a + 4 > self.b.len() {
            return None;
        }
        let s = std::str::from_utf8(&self.b[a..a + 4]).ok()?;
        if !s.bytes().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(s, 16).ok()
    }

    /// Literal de texto. Devolve o texto (substituto isolado vira U+FFFD e liga
    /// a flag `substituto`, recusado no passo 5) e a forma WTF-8 exata, usada
    /// para comparar chaves (passo 4).
    fn texto(&mut self) -> Result<(String, Vec<u8>), String> {
        self.i += 1;
        let mut s = String::new();
        let mut w: Vec<u8> = Vec::new();
        let inicio_cru = |s: &mut String, w: &mut Vec<u8>, pedaco: &[u8]| {
            // pedaço vem de &str válido e é cortado em fronteira ASCII
            s.push_str(std::str::from_utf8(pedaco).unwrap_or("\u{FFFD}"));
            w.extend_from_slice(pedaco);
        };
        loop {
            let a = self.i;
            while self.i < self.b.len() && self.b[self.i] >= 0x20 && self.b[self.i] != b'"' && self.b[self.i] != b'\\' {
                self.i += 1;
            }
            if self.i > a {
                let pedaco = &self.b[a..self.i];
                inicio_cru(&mut s, &mut w, pedaco);
            }
            if self.i >= self.b.len() {
                return Err(invalido());
            }
            match self.b[self.i] {
                b'"' => {
                    self.i += 1;
                    return Ok((s, w));
                }
                b'\\' => {
                    self.i += 1;
                    if self.i >= self.b.len() {
                        return Err(invalido());
                    }
                    let e = self.b[self.i];
                    self.i += 1;
                    let c = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let u = self.hex4(self.i).ok_or_else(invalido)?;
                            self.i += 4;
                            let mut cp = u;
                            if (0xD800..=0xDBFF).contains(&u)
                                && self.i + 1 < self.b.len()
                                && self.b[self.i] == b'\\'
                                && self.b[self.i + 1] == b'u'
                            {
                                let u2 = self.hex4(self.i + 2).ok_or_else(invalido)?;
                                if (0xDC00..=0xDFFF).contains(&u2) {
                                    cp = 0x10000 + ((u - 0xD800) << 10) + (u2 - 0xDC00);
                                    self.i += 6;
                                }
                            }
                            match char::from_u32(cp) {
                                Some(ch) => ch,
                                None => {
                                    // substituto isolado: WTF-8 de 3 bytes
                                    self.substituto = true;
                                    s.push('\u{FFFD}');
                                    w.extend_from_slice(&[
                                        0xE0 | (cp >> 12) as u8,
                                        0x80 | ((cp >> 6) & 0x3F) as u8,
                                        0x80 | (cp & 0x3F) as u8,
                                    ]);
                                    continue;
                                }
                            }
                        }
                        _ => return Err(invalido()),
                    };
                    s.push(c);
                    let mut buf = [0u8; 4];
                    w.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
                _ => return Err(invalido()), // controle cru < U+0020
            }
        }
    }

    fn numero(&mut self) -> Result<Valor, String> {
        let b = self.b;
        let a = self.i;
        let mut i = a;
        let digito = |i: usize| i < b.len() && b[i].is_ascii_digit();
        if b[i] == b'-' {
            i += 1;
        }
        if i < b.len() && (b'1'..=b'9').contains(&b[i]) {
            i += 1;
            while digito(i) {
                i += 1;
            }
        } else if i < b.len() && b[i] == b'0' {
            i += 1;
        } else {
            return Err(invalido());
        }
        let mut flutuante = false;
        if i < b.len() && b[i] == b'.' && digito(i + 1) {
            flutuante = true;
            i += 2;
            while digito(i) {
                i += 1;
            }
        }
        if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
            let mut j = i + 1;
            if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
                j += 1;
            }
            if digito(j) {
                while digito(j) {
                    j += 1;
                }
                flutuante = true;
                i = j;
            }
        }
        self.i = i;
        let lexema = std::str::from_utf8(&b[a..i]).unwrap_or("");
        let prefixo: String = lexema.chars().take(20).collect();
        if !flutuante {
            let digitos = lexema.trim_start_matches('-');
            let v = if digitos.len() > 16 { None } else { lexema.parse::<i64>().ok() };
            match v {
                Some(v) if v.abs() <= LIMITE => Ok(Valor::Int(v)),
                _ => Err(format!("R-5.16: inteiro fora de ±(2^53 - 1): {prefixo}…")),
            }
        } else {
            let v: f64 = lexema.parse().map_err(|_| invalido())?;
            if !v.is_finite() || v.abs() > LIMITE as f64 {
                return Err(format!("R-5.16: número fora de ±(2^53 - 1) ou não finito: {prefixo}"));
            }
            Ok(Valor::Float(v))
        }
    }
}

// ---------------------------------------------------------------- §2.2 json_canonico

/// `repr` de float do Python (§2.2), para |x| ≤ 2^53 − 1 finito.
pub fn float_repr(x: f64) -> String {
    if x == 0.0 {
        return if x.is_sign_negative() { "-0.0".into() } else { "0.0".into() };
    }
    let sinal = if x < 0.0 { "-" } else { "" };
    let e = format!("{:e}", x.abs()); // dígitos mais curtos de ida e volta
    let (mant, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let exp: i64 = exp.parse().unwrap_or(0);
    let mut d: String = mant.chars().filter(|c| *c != '.').collect();
    while d.len() > 1 && d.ends_with('0') {
        d.pop();
    }
    let n = d.len() as i64;
    let p = exp + 1;
    let corpo = if p <= -4 {
        let mut s = d[..1].to_string();
        if n > 1 {
            s.push('.');
            s.push_str(&d[1..]);
        }
        let q = p - 1;
        s.push_str(&format!("e{}{:02}", if q < 0 { '-' } else { '+' }, q.abs()));
        s
    } else if p <= 0 {
        format!("0.{}{}", "0".repeat((-p) as usize), d)
    } else if p >= n {
        format!("{}{}.0", d, "0".repeat((p - n) as usize))
    } else {
        format!("{}.{}", &d[..p as usize], &d[p as usize..])
    };
    format!("{sinal}{corpo}")
}

fn canonico_em(v: &Valor, out: &mut Vec<u8>) {
    match v {
        Valor::Nulo => out.extend_from_slice(b"null"),
        Valor::Bool(true) => out.extend_from_slice(b"true"),
        Valor::Bool(false) => out.extend_from_slice(b"false"),
        Valor::Int(i) => out.extend_from_slice(i.to_string().as_bytes()),
        Valor::Float(f) => out.extend_from_slice(float_repr(*f).as_bytes()),
        Valor::Texto(s) => texto_canonico(s, out),
        Valor::Lista(l) => {
            out.push(b'[');
            for (i, x) in l.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                canonico_em(x, out);
            }
            out.push(b']');
        }
        Valor::Objeto(o) => {
            let mut pares: Vec<&(String, Valor)> = o.0.iter().collect();
            pares.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
            out.push(b'{');
            for (i, (k, x)) in pares.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                texto_canonico(k, out);
                out.push(b':');
                canonico_em(x, out);
            }
            out.push(b'}');
        }
    }
}

fn texto_canonico(s: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for c in s.chars() {
        match c {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            '\u{8}' => out.extend_from_slice(b"\\b"),
            '\u{c}' => out.extend_from_slice(b"\\f"),
            c if (c as u32) < 0x20 => out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes()),
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

/// §2.2: `json.dumps(v, ensure_ascii=False, sort_keys=True, separators=(",", ":"))`.
pub fn json_canonico(v: &Valor) -> Vec<u8> {
    let mut out = Vec::new();
    canonico_em(v, &mut out);
    out
}

/// §2.3.
pub fn iguais(a: &Valor, b: &Valor) -> bool {
    json_canonico(a) == json_canonico(b)
}

// ---------------------------------------------------------------- §2.4 repr_py

fn escapa_nao_ascii(c: u32) -> bool {
    matches!(c,
        0x80..=0xA0 | 0xAD | 0x1680 | 0x2000..=0x200F | 0x2028..=0x202F | 0x205F..=0x206F
        | 0x3000 | 0xFEFF | 0xE000..=0xF8FF | 0xF0000..=0x10FFFF)
}

/// `repr` de `str` do Python, com a tabela fixa J-R1 para não-ASCII.
pub fn repr_texto(s: &str) -> String {
    let aspa = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
    let mut out = String::new();
    out.push(aspa);
    for c in s.chars() {
        let u = c as u32;
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c == aspa => {
                out.push('\\');
                out.push(c);
            }
            _ if u < 0x20 || u == 0x7F => out.push_str(&format!("\\x{u:02x}")),
            _ if u < 0x80 => out.push(c),
            _ if escapa_nao_ascii(u) => {
                if u <= 0xFF {
                    out.push_str(&format!("\\x{u:02x}"));
                } else if u <= 0xFFFF {
                    out.push_str(&format!("\\u{u:04x}"));
                } else {
                    out.push_str(&format!("\\U{u:08x}"));
                }
            }
            _ => out.push(c),
        }
    }
    out.push(aspa);
    out
}

/// §2.4: `repr()` do Python para valores J-D.
pub fn repr_py(v: &Valor) -> String {
    match v {
        Valor::Nulo => "None".into(),
        Valor::Bool(true) => "True".into(),
        Valor::Bool(false) => "False".into(),
        Valor::Int(i) => i.to_string(),
        Valor::Float(f) => float_repr(*f),
        Valor::Texto(s) => repr_texto(s),
        Valor::Lista(l) => {
            let itens: Vec<String> = l.iter().map(repr_py).collect();
            format!("[{}]", itens.join(", "))
        }
        Valor::Objeto(o) => {
            let itens: Vec<String> = o.iter().map(|(k, x)| format!("{}: {}", repr_texto(k), repr_py(x))).collect();
            format!("{{{}}}", itens.join(", "))
        }
    }
}

/// repr de uma opção (`None` se ausente), como `{x!r}` de `d.get(k)`.
pub fn repr_opt(v: Option<&Valor>) -> String {
    v.map(repr_py).unwrap_or_else(|| "None".into())
}

// ---------------------------------------------------------------- testes

#[cfg(test)]
mod testes {
    use super::*;

    fn c(v: &Valor) -> String {
        String::from_utf8(json_canonico(v)).unwrap()
    }
    fn e(s: &str) -> Result<Valor, String> {
        json_estrito(s.as_bytes())
    }
    fn erro(s: &[u8]) -> String {
        json_estrito(s).unwrap_err()
    }

    #[test]
    fn vetores_float_2_2() {
        let casos = [
            (1.0, "1.0"), (1.5, "1.5"), (0.1, "0.1"), (0.0001, "0.0001"), (1e-05, "1e-05"),
            (2.5e-05, "2.5e-05"), (1.234e-07, "1.234e-07"), (5e-324, "5e-324"),
            (123456.789, "123456.789"), (1e15, "1000000000000000.0"),
            (9007199254740991.0, "9007199254740991.0"), (0.1 + 0.2, "0.30000000000000004"),
            (0.0, "0.0"), (-0.0, "-0.0"), (-1.5, "-1.5"), (100.0, "100.0"),
        ];
        for (x, s) in casos {
            assert_eq!(float_repr(x), s, "{x}");
        }
    }

    #[test]
    fn vetor_objeto_2_2() {
        let v = e("{\"z\":0.0001,\"y\":1e-05,\"x\":-0.0,\"é\":\"\u{2028}\\u007f\\u0001\",\"B\":[1,2.0]}").unwrap();
        assert_eq!(c(&v), "{\"B\":[1,2.0],\"x\":-0.0,\"y\":1e-05,\"z\":0.0001,\"é\":\"\u{2028}\u{7f}\\u0001\"}");
    }

    #[test]
    fn ordem_de_chaves_codigo() {
        let v = e("{\"é\":1,\"e\u{301}\":2,\"e\":3,\"Z\":4,\"a\":5}").unwrap();
        assert_eq!(c(&v), "{\"Z\":4,\"a\":5,\"e\":3,\"e\u{301}\":2,\"é\":1}");
    }

    #[test]
    fn escapes_canonicos() {
        let v = Valor::texto("\"\\\n\r\t\u{8}\u{c}\u{1f}é");
        assert_eq!(c(&v), "\"\\\"\\\\\\n\\r\\t\\b\\f\\u001fé\"");
    }

    #[test]
    fn iguais_tipos() {
        let (t, um, umf) = (Valor::Bool(true), Valor::Int(1), Valor::Float(1.0));
        assert!(!iguais(&t, &um) && !iguais(&um, &umf) && !iguais(&t, &umf));
        assert!(!iguais(&Valor::Float(0.0), &Valor::Float(-0.0)));
        let a = e("{\"a\":1,\"b\":[1,2]}").unwrap();
        let b = e("{\"b\":[1,2],\"a\":1}").unwrap();
        assert!(iguais(&a, &b));
        let d = e("{\"b\":[2,1],\"a\":1}").unwrap();
        assert!(!iguais(&a, &d));
    }

    #[test]
    fn limites_numericos() {
        assert!(matches!(e("9007199254740991"), Ok(Valor::Int(9007199254740991))));
        assert!(matches!(e("-9007199254740991"), Ok(Valor::Int(-9007199254740991))));
        assert_eq!(e("9007199254740992").unwrap_err(), "R-5.16: inteiro fora de ±(2^53 - 1): 9007199254740992…");
        assert_eq!(e("12345678901234567").unwrap_err(), "R-5.16: inteiro fora de ±(2^53 - 1): 12345678901234567…");
        assert_eq!(e("00000000000000000001").unwrap_err(), "R-5.16: JSON inválido");
        assert!(matches!(e("9007199254740991.0"), Ok(Valor::Float(_))));
        assert_eq!(
            e("9007199254740992.0").unwrap_err(),
            "R-5.16: número fora de ±(2^53 - 1) ou não finito: 9007199254740992.0"
        );
        assert_eq!(
            e("9007199254740991.5").unwrap_err(),
            "R-5.16: número fora de ±(2^53 - 1) ou não finito: 9007199254740991.5"
        );
        assert_eq!(e("1e400").unwrap_err(), "R-5.16: número fora de ±(2^53 - 1) ou não finito: 1e400");
        assert!(matches!(e("1e-400"), Ok(Valor::Float(f)) if f == 0.0));
        assert!(matches!(e("-0"), Ok(Valor::Int(0))));
        let longo = format!("1{}", "0".repeat(30));
        assert_eq!(e(&longo).unwrap_err(), "R-5.16: inteiro fora de ±(2^53 - 1): 10000000000000000000…");
        // 17 dígitos antes de um '.' sem fração: o inteiro é lido e recusado primeiro
        assert!(e("12345678901234567.").unwrap_err().starts_with("R-5.16: inteiro"));
    }

    #[test]
    fn constantes() {
        assert_eq!(e("NaN").unwrap_err(), "R-5.16: NaN não é número JSON finito");
        assert_eq!(e("[Infinity]").unwrap_err(), "R-5.16: Infinity não é número JSON finito");
        assert_eq!(e("-Infinity").unwrap_err(), "R-5.16: -Infinity não é número JSON finito");
        assert_eq!(e("-NaN").unwrap_err(), "R-5.16: JSON inválido");
    }

    #[test]
    fn repetida_e_substituto() {
        // número inválido antes de chave repetida
        assert!(e("{\"a\":1,\"a\":1e400}").unwrap_err().contains("não finito"));
        assert_eq!(e("{\"a\":1,\"a\":1}").unwrap_err(), "R-5.16: objeto com chave repetida (C4-4)");
        assert_eq!(e("{\"a\":1,\"\\u0061\":2}").unwrap_err(), "R-5.16: objeto com chave repetida (C4-4)");
        assert_eq!(
            e("{\"\u{1F600}\":1,\"\\ud83d\\ude00\":2}").unwrap_err(),
            "R-5.16: objeto com chave repetida (C4-4)"
        );
        assert_eq!(e("\"\\ud800\"").unwrap_err(), "R-5.16: texto com substituto UTF-16 isolado");
        assert_eq!(e("{\"\\ud800\":1,\"\\ud800\":2}").unwrap_err(), "R-5.16: objeto com chave repetida (C4-4)");
        assert_eq!(e("{\"\\ud800\":1,\"\\udc00\":2}").unwrap_err(), "R-5.16: texto com substituto UTF-16 isolado");
        assert!(matches!(e("\"\\ud83d\\ude00\""), Ok(Valor::Texto(s)) if s == "\u{1F600}"));
        assert_eq!(e("\"\\uD83D\\uDE00\"").unwrap().como_texto(), Some("\u{1F600}"));
        // substituto alto seguido de escape não-baixo: os dois ficam (alto isolado)
        assert_eq!(e("\"\\ud800\\u0041\"").unwrap_err(), "R-5.16: texto com substituto UTF-16 isolado");
    }

    #[test]
    fn profundidade_64_65() {
        let ok = format!("{}{}", "[".repeat(64), "]".repeat(64));
        assert!(e(&ok).is_ok());
        let ruim = format!("{}{}", "[".repeat(65), "]".repeat(65));
        assert_eq!(e(&ruim).unwrap_err(), "R-5.16: mais de 64 níveis de aninhamento");
        // vale também para JSON inválido
        assert_eq!(e(&"[".repeat(65)).unwrap_err(), "R-5.16: mais de 64 níveis de aninhamento");
        // colchetes dentro de texto não contam
        let dentro = format!("[\"{}\"]", "[".repeat(100));
        assert!(e(&dentro).is_ok());
        // texto sem fecho: os colchetes depois da aspa contam
        let sem_fecho = format!("\"{}", "[".repeat(65));
        assert_eq!(e(&sem_fecho).unwrap_err(), "R-5.16: mais de 64 níveis de aninhamento");
        // aspa escapada no fim reabre a varredura
        assert_eq!(profundidade("\"a\\\"[["), 2);
        assert_eq!(profundidade("\"ab\"[\"x"), 1);
        assert_eq!(profundidade("]]][["), 0);
    }

    #[test]
    fn invalidos() {
        for s in [
            "\u{feff}{}", "{} x", "[1,]", "{\"a\":1,}", "\"a\u{1}b\"", "", " ", "01", "1.", "1e", "+1",
            "'a'", "{a:1}", "[1 2]", "\"\\x\"", "\"\\u12\"", "tru", "nul", "\"abc", "{\"a\" 1}", "\u{a0}1",
        ] {
            assert_eq!(e(s).unwrap_err(), "R-5.16: JSON inválido", "{s:?}");
        }
        assert_eq!(erro(b"\xff"), "R-5.16: UTF-8 inválido");
        assert_eq!(erro(b"\"\xc0\xaf\""), "R-5.16: UTF-8 inválido");
        assert_eq!(erro(b"\"\xed\xa0\x80\""), "R-5.16: UTF-8 inválido");
        assert_eq!(erro(b"\"\xe2\x82\""), "R-5.16: UTF-8 inválido");
        assert!(e(" \t\r\n{\"a\" : [ 1 , 2.5e3 , true , false , null , \"\\/\\b\" ] }\n").is_ok());
        assert!(matches!(e("1E2"), Ok(Valor::Float(f)) if f == 100.0));
    }

    #[test]
    fn repr_textos() {
        assert_eq!(repr_texto("a'b"), "\"a'b\"");
        assert_eq!(repr_texto("a\"b"), "'a\"b'");
        assert_eq!(repr_texto("a'\"b"), "'a\\'\"b'");
        assert_eq!(repr_texto("\t\x01\x7f\\"), "'\\t\\x01\\x7f\\\\'");
        assert_eq!(repr_texto("\n\r"), "'\\n\\r'");
        assert_eq!(repr_texto("é\u{a0}\u{2028}\u{feff}\u{10ffff}"), "'é\\xa0\\u2028\\ufeff\\U0010ffff'");
        let v = e("{\"k\":[null,true,false,1,1.5,\"x\"],\"b\":{}}").unwrap();
        assert_eq!(repr_py(&v), "{'k': [None, True, False, 1, 1.5, 'x'], 'b': {}}");
    }

    #[test]
    fn termos() {
        assert_eq!(strip("\u{1c} x\u{3000}"), "x");
        assert!(!id_valido(Some(&Valor::texto(" \u{85}"))));
        assert!(id_valido(Some(&Valor::texto(" a"))));
        assert!(!id_valido(Some(&Valor::Int(1))));
        assert!(e_data("2024-02-29") && !e_data("2023-02-29") && !e_data("2026-02-30"));
        assert!(!e_data("0000-01-01") && !e_data("2026-1-01") && !e_data("2026-01-01\n"));
        assert!(e_data("9999-12-31") && !e_data("2026-13-01") && !e_data("2026-00-10"));
        assert!(e_hex64(&"a".repeat(64)) && !e_hex64(&"A".repeat(64)) && !e_hex64(&"a".repeat(63)));
    }
}
