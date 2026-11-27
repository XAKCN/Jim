//! §2.5–§2.7 e §6.1 de contract/SEMANTICA.md: SHA-256, caminhos, hash do
//! fecho e inventário. Só `inventariar` lê o sistema de arquivos.

use crate::json::repr_texto;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// SHA-256 hex minúsculo.
pub fn sha256_hex(dados: &[u8]) -> String {
    let d = Sha256::digest(dados);
    let mut s = String::with_capacity(64);
    for b in d.iter() {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// `cadeia.hash_linha`: SHA-256 dos bytes UTF-8 do texto.
pub fn hash_linha(texto: &str) -> String {
    sha256_hex(texto.as_bytes())
}

/// §2.5.
pub fn validar_caminho(c: &str) -> Result<(), String> {
    let invalido = || format!("J-C1: caminho relativo inválido: {}", repr_texto(c));
    if c.is_empty() || c.starts_with('/') || c.contains('\\') || c.chars().any(|x| (x as u32) < 0x20 || x == '\u{7f}') {
        return Err(invalido());
    }
    if c.split('/').any(|s| s.is_empty() || s == "." || s == "..") {
        return Err(invalido());
    }
    if c == "candidato" || c == "contrato" {
        return Err(format!("J-C2: caminho reservado: {}", repr_texto(c)));
    }
    Ok(())
}

/// Fórmula de §2.6 sem validação (aceita vazio): hex de
/// sha256(concatenação ordenada de "caminho hash\n").
pub fn arvore(pares: &[(String, String)]) -> String {
    let mut v: Vec<&(String, String)> = pares.iter().collect();
    v.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut texto = String::new();
    for (p, h) in v {
        texto.push_str(p);
        texto.push(' ');
        texto.push_str(h);
        texto.push('\n');
    }
    sha256_hex(texto.as_bytes())
}

/// {caminho válido: hex64}, sem exigir não vazio.
pub fn validar_inventario(arquivos: &[(String, String)]) -> Result<(), String> {
    for (p, h) in arquivos {
        validar_caminho(p)?;
        if !crate::json::e_hex64(h) {
            return Err(format!("J-F2: hash inválido em {}", repr_texto(p)));
        }
    }
    Ok(())
}

/// §2.6.
pub fn hash_fecho(arquivos: &[(String, String)]) -> Result<String, String> {
    if arquivos.is_empty() {
        return Err("J-F1: fecho vazio".into());
    }
    validar_inventario(arquivos)?;
    Ok(format!("sha256:{}", arvore(arquivos)))
}

/// §2.7.
pub fn inventariar(raiz: &str, caminhos: &[String]) -> Result<Vec<(String, String)>, String> {
    let base: PathBuf = match std::fs::canonicalize(raiz) {
        Ok(b) if b.is_dir() => b,
        _ => return Err(format!("J-C8: raiz não é diretório: {}", repr_texto(raiz))),
    };
    let mut vistos = HashSet::new();
    for c in caminhos {
        validar_caminho(c)?;
        if !vistos.insert(c.as_str()) {
            return Err(format!("J-C3: caminho repetido: {}", repr_texto(c)));
        }
    }
    let mut out = Vec::with_capacity(caminhos.len());
    for c in caminhos {
        out.push((c.clone(), ler_um(&base, c)?));
    }
    Ok(out)
}

fn ler_um(base: &Path, c: &str) -> Result<String, String> {
    let r = repr_texto(c);
    let partes: Vec<&str> = c.split('/').collect();
    let mut atual = base.to_path_buf();
    for (i, parte) in partes.iter().enumerate() {
        atual.push(parte);
        let meta = match std::fs::symlink_metadata(&atual) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(format!("J-C4: arquivo ausente: {r}")),
            // ENOTDIR: componente intermediário não é diretório → o caminho não existe.
            Err(_) if i + 1 < partes.len() => return Err(format!("J-C4: arquivo ausente: {r}")),
            Err(_) => return Err(format!("J-C7: arquivo ilegível: {r}")),
        };
        if meta.file_type().is_symlink() {
            return Err(format!("J-C5: symlink recusado: {r}"));
        }
        if i + 1 < partes.len() {
            if !meta.is_dir() {
                return Err(format!("J-C4: arquivo ausente: {r}"));
            }
        } else if !meta.is_file() {
            return Err(format!("J-C6: não é arquivo regular: {r}"));
        }
    }
    use std::io::Read;
    let mut f = std::fs::File::open(&atual).map_err(|_| format!("J-C7: arquivo ilegível: {r}"))?;
    // Confere de novo no descritor aberto (troca entre lstat e open).
    match f.metadata() {
        Ok(m) if m.is_file() => {}
        _ => return Err(format!("J-C6: não é arquivo regular: {r}")),
    }
    let mut dados = Vec::new();
    f.read_to_end(&mut dados).map_err(|_| format!("J-C7: arquivo ilegível: {r}"))?;
    Ok(sha256_hex(&dados))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn d(pares: &[(&str, String)]) -> Vec<(String, String)> {
        pares.iter().map(|(a, b)| (a.to_string(), b.clone())).collect()
    }

    #[test]
    fn vetores_hash_fecho_v5_alpha() {
        let a = d(&[("b.py", "0".repeat(64)), ("a/x.py", "f".repeat(64))]);
        assert_eq!(hash_fecho(&a).unwrap(), "sha256:9ba2313d796760aa2f69f6ef98ec9f55aab8d2b35a6cbc0dac9184a5f455f44c");
        let b = d(&[
            ("é.py", "1".repeat(64)),
            ("Z.py", "2".repeat(64)),
            ("a.py", "3".repeat(64)),
            ("a b.py", "4".repeat(64)),
        ]);
        assert_eq!(hash_fecho(&b).unwrap(), "sha256:6e64fff17930dea8ed52dd5b1e2eaaceb182c5509555a5004c9a561cb429aa4f");
        assert_eq!(arvore(&[]), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn sha256_e_linha() {
        assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(hash_linha("ação"), sha256_hex("ação".as_bytes()));
    }

    #[test]
    fn hash_fecho_recusas() {
        assert_eq!(hash_fecho(&[]).unwrap_err(), "J-F1: fecho vazio");
        assert_eq!(hash_fecho(&d(&[("a.py", "A".repeat(64))])).unwrap_err(), "J-F2: hash inválido em 'a.py'");
        assert_eq!(hash_fecho(&d(&[("./a.py", "a".repeat(64))])).unwrap_err(), "J-C1: caminho relativo inválido: './a.py'");
        assert_eq!(hash_fecho(&d(&[("contrato", "a".repeat(64))])).unwrap_err(), "J-C2: caminho reservado: 'contrato'");
    }

    #[test]
    fn caminhos() {
        for ok in ["a.py", "a/b/c.py", "a b.py", "é.py", "...", ".oculto", "a/.b"] {
            assert!(validar_caminho(ok).is_ok(), "{ok}");
        }
        for ruim in ["", "/a", "a//b", "./a", "a/./b", "a/../b", "..", ".", "a/", "a\\b", "a\nb", "a\u{7f}", "a\u{1}"] {
            assert!(validar_caminho(ruim).unwrap_err().starts_with("J-C1: "), "{ruim:?}");
        }
        assert_eq!(validar_caminho("candidato").unwrap_err(), "J-C2: caminho reservado: 'candidato'");
        assert!(validar_caminho("x/candidato").is_ok());
    }

    #[test]
    fn inventario_em_disco() {
        let base = std::env::temp_dir().join(format!("jim_inv_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("d/e")).unwrap();
        std::fs::write(base.join("a.py"), b"abc").unwrap();
        std::fs::write(base.join("d/e/x.txt"), b"").unwrap();
        let raiz = base.to_str().unwrap();
        let cs = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let r = inventariar(raiz, &cs(&["d/e/x.txt", "a.py"])).unwrap();
        assert_eq!(r[0].0, "d/e/x.txt");
        assert_eq!(r[1].1, sha256_hex(b"abc"));
        assert_eq!(inventariar(raiz, &cs(&["a.py", "a.py"])).unwrap_err(), "J-C3: caminho repetido: 'a.py'");
        assert_eq!(inventariar(raiz, &cs(&["b.py"])).unwrap_err(), "J-C4: arquivo ausente: 'b.py'");
        assert_eq!(inventariar(raiz, &cs(&["a.py/x"])).unwrap_err(), "J-C4: arquivo ausente: 'a.py/x'");
        assert_eq!(inventariar(raiz, &cs(&["d"])).unwrap_err(), "J-C6: não é arquivo regular: 'd'");
        assert!(inventariar(raiz, &cs(&["../a.py"])).unwrap_err().starts_with("J-C1: "));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(base.join("a.py"), base.join("l.py")).unwrap();
            std::os::unix::fs::symlink(base.join("d"), base.join("ld")).unwrap();
            assert_eq!(inventariar(raiz, &cs(&["l.py"])).unwrap_err(), "J-C5: symlink recusado: 'l.py'");
            assert_eq!(inventariar(raiz, &cs(&["ld/e/x.txt"])).unwrap_err(), "J-C5: symlink recusado: 'ld/e/x.txt'");
            // symlink na própria raiz é aceito
            let lr = std::env::temp_dir().join(format!("jim_inv_lr_{}", std::process::id()));
            let _ = std::fs::remove_file(&lr);
            std::os::unix::fs::symlink(&base, &lr).unwrap();
            assert!(inventariar(lr.to_str().unwrap(), &cs(&["a.py"])).is_ok());
            let _ = std::fs::remove_file(&lr);
        }
        assert!(inventariar(base.join("a.py").to_str().unwrap(), &[]).unwrap_err().starts_with("J-C8: "));
        assert!(inventariar(base.join("nada").to_str().unwrap(), &[]).unwrap_err().starts_with("J-C8: "));
        let _ = std::fs::remove_dir_all(&base);
    }
}
