//! §2.5–§2.7 e §6.1 de contract/SEMANTICA.md: SHA-256, caminhos, hash do
//! fecho e inventário. Só `inventariar` lê o sistema de arquivos.

use crate::json::repr_texto;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io;
#[cfg(unix)]
use std::{
    ffi::{CStr, CString},
    fs::{File, OpenOptions},
    io::Read,
    os::{fd::{AsRawFd, FromRawFd}, unix::fs::OpenOptionsExt},
};

#[cfg(any(target_os = "linux", target_os = "android"))]
const ABRIR_DIRETORIO: libc::c_int = libc::O_PATH;
#[cfg(target_vendor = "apple")]
const ABRIR_DIRETORIO: libc::c_int = libc::O_SEARCH;
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android", target_vendor = "apple"))))]
const ABRIR_DIRETORIO: libc::c_int = libc::O_RDONLY | libc::O_NONBLOCK;

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
    inventariar_apos_resolver(raiz, caminhos, || {})
}

fn inventariar_apos_resolver(
    raiz: &str,
    caminhos: &[String],
    apos_resolver: impl FnOnce(),
) -> Result<Vec<(String, String)>, String> {
    let erro_raiz = |erro: io::Error| erro_da_raiz(raiz, erro.kind());
    // A abertura resolve a raiz (inclusive symlinks) e fixa seu descritor.
    #[cfg(unix)]
    let base = OpenOptions::new()
        .read(true)
        .custom_flags(ABRIR_DIRETORIO | libc::O_DIRECTORY)
        .open(raiz)
        .map_err(erro_raiz)?;
    #[cfg(not(unix))]
    if !std::fs::metadata(raiz).map_err(erro_raiz)?.is_dir() {
        return Err(erro_da_raiz(raiz, io::ErrorKind::NotADirectory));
    }
    apos_resolver();
    let mut vistos = HashSet::new();
    for c in caminhos {
        validar_caminho(c)?;
        if !vistos.insert(c.as_str()) {
            return Err(format!("J-C3: caminho repetido: {}", repr_texto(c)));
        }
    }
    #[cfg(unix)]
    {
        caminhos.iter().map(|c| Ok((c.clone(), ler_um(&base, c, || {})?))).collect()
    }
    #[cfg(not(unix))]
    {
        match caminhos.first() {
            None => Ok(Vec::new()),
            Some(c) => Err(format!("J-C7: abertura segura indisponível: {}", repr_texto(c))),
        }
    }
}

fn erro_da_raiz(raiz: &str, tipo: io::ErrorKind) -> String {
    let raiz = repr_texto(raiz);
    match tipo {
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => {
            format!("J-C8: raiz não é diretório: {raiz}")
        }
        _ => format!("J-C7: raiz ilegível: {raiz}"),
    }
}

#[cfg(unix)]
fn tipo_em(dir: &File, nome: &CStr) -> io::Result<libc::mode_t> {
    let mut meta = std::mem::MaybeUninit::<libc::stat>::uninit();
    // fstatat só inicializa meta quando retorna 0.
    let rc = unsafe { libc::fstatat(dir.as_raw_fd(), nome.as_ptr(), meta.as_mut_ptr(), libc::AT_SYMLINK_NOFOLLOW) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { meta.assume_init() }.st_mode & libc::S_IFMT)
}

#[cfg(unix)]
fn abrir_em(dir: &File, nome: &CStr, flags: libc::c_int) -> io::Result<File> {
    // O descritor retornado pertence a esta chamada e passa a ser gerido por File.
    let fd = unsafe { libc::openat(dir.as_raw_fd(), nome.as_ptr(), flags | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(unix)]
fn erro_de_caminho(e: io::Error, r: &str) -> String {
    if e.raw_os_error() == Some(libc::ELOOP) {
        return format!("J-C5: symlink recusado: {r}");
    }
    match e.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => format!("J-C4: arquivo ausente: {r}"),
        _ => format!("J-C7: arquivo ilegível: {r}"),
    }
}

#[cfg(unix)]
fn ler_um(base: &File, c: &str, antes_de_abrir: impl FnOnce()) -> Result<String, String> {
    let r = repr_texto(c);
    let ausente = || format!("J-C4: arquivo ausente: {r}");
    let symlink = || format!("J-C5: symlink recusado: {r}");
    let irregular = || format!("J-C6: não é arquivo regular: {r}");
    let ilegivel = || format!("J-C7: arquivo ilegível: {r}");
    let erro_de_caminho = |e| erro_de_caminho(e, &r);
    let (pais, nome) = c.rsplit_once('/').map_or((None, c), |(p, n)| (Some(p), n));
    let mut pai = None;
    for parte in pais.into_iter().flat_map(|p| p.split('/')) {
        let dir = pai.as_ref().unwrap_or(base);
        let parte = CString::new(parte).map_err(|_| ilegivel())?;
        match tipo_em(dir, &parte).map_err(erro_de_caminho)? {
            libc::S_IFLNK => return Err(symlink()),
            libc::S_IFDIR => {}
            _ => return Err(ausente()),
        }
        let aberto = abrir_em(dir, &parte, ABRIR_DIRETORIO).map_err(erro_de_caminho)?;
        let meta = aberto.metadata().map_err(|_| ilegivel())?;
        if meta.file_type().is_symlink() {
            return Err(symlink());
        }
        if !meta.is_dir() {
            return Err(ausente());
        }
        pai = Some(aberto);
    }
    let dir = pai.as_ref().unwrap_or(base);
    let nome = CString::new(nome).map_err(|_| ilegivel())?;
    match tipo_em(dir, &nome).map_err(erro_de_caminho)? {
        libc::S_IFLNK => return Err(symlink()),
        libc::S_IFREG => {}
        _ => return Err(irregular()),
    }
    antes_de_abrir();
    let mut f = abrir_em(dir, &nome, libc::O_RDONLY | libc::O_NONBLOCK).map_err(erro_de_caminho)?;
    if !f.metadata().map_err(|_| ilegivel())?.is_file() {
        return Err(irregular());
    }
    let mut dados = Vec::new();
    f.read_to_end(&mut dados).map_err(|_| ilegivel())?;
    Ok(sha256_hex(&dados))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn inventario_mantem_raiz_apos_mover_ancestral() {
        use std::fs;

        let pasta = std::env::temp_dir().join(format!("jim_inv_ancestral_{}", std::process::id()));
        let ancestral = pasta.join("ancestral");
        let raiz = ancestral.join("raiz");
        fs::create_dir_all(raiz.join("sub")).unwrap();
        fs::write(raiz.join("a.txt"), b"primeiro").unwrap();
        fs::write(raiz.join("sub/b.txt"), b"segundo").unwrap();
        let caminhos = ["a.txt".into(), "sub/b.txt".into()];
        let resultado = inventariar_apos_resolver(raiz.to_str().unwrap(), &caminhos, || {
            fs::rename(&ancestral, pasta.join("movido")).unwrap();
        });
        fs::remove_dir_all(&pasta).unwrap();

        assert_eq!(resultado, Ok(d(&[
            ("a.txt", sha256_hex(b"primeiro")),
            ("sub/b.txt", sha256_hex(b"segundo")),
        ])));
    }

    #[cfg(unix)]
    #[test]
    fn inventario_distingue_raiz_inacessivel() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let pasta = std::env::temp_dir().join(format!("jim_inv_raiz_ilegivel_{}", std::process::id()));
        let ancestral = pasta.join("restrito");
        let raiz = ancestral.join("raiz");
        fs::create_dir_all(&raiz).unwrap();
        let permissoes = fs::metadata(&ancestral).unwrap().permissions();
        fs::set_permissions(&ancestral, fs::Permissions::from_mode(0o600)).unwrap();
        let acesso = fs::metadata(&raiz);
        let raiz_texto = raiz.to_str().unwrap();
        let resultado = inventariar(raiz_texto, &[]);
        fs::set_permissions(&ancestral, permissoes).unwrap();
        fs::remove_dir_all(&pasta).unwrap();

        let esperado = format!("J-C7: raiz ilegível: {}", repr_texto(raiz_texto));
        // Executores privilegiados podem atravessar diretórios sem bit de busca.
        if let Err(erro) = acesso {
            assert_eq!(erro.kind(), io::ErrorKind::PermissionDenied);
            assert_eq!(resultado, Err(esperado.clone()));
        }
        for codigo in [libc::EACCES, libc::EPERM] {
            let tipo = io::Error::from_raw_os_error(codigo).kind();
            assert_eq!(erro_da_raiz(raiz_texto, tipo), esperado);
        }
    }

    #[cfg(unix)]
    #[test]
    fn inventario_recusa_symlink_trocado_antes_do_open() {
        use std::fs;
        use std::os::unix::fs::symlink;

        let pasta = std::env::temp_dir().join(format!("jim_inv_troca_{}", std::process::id()));
        fs::create_dir_all(pasta.join("raiz/a")).unwrap();
        let raiz = pasta.join("raiz");
        let arquivo = raiz.join("a/t.py");
        let alvo = pasta.join("alvo");
        fs::write(&arquivo, b"original").unwrap();
        fs::write(&alvo, b"fora da raiz").unwrap();
        let base = File::open(&raiz).unwrap();
        let resultado = ler_um(&base, "a/t.py", || {
            fs::remove_file(&arquivo).unwrap();
            symlink(&alvo, &arquivo).unwrap();
        });
        fs::remove_dir_all(&pasta).unwrap();
        assert_eq!(resultado, Err("J-C5: symlink recusado: 'a/t.py'".into()));
    }

    #[cfg(unix)]
    #[test]
    fn inventario_distingue_componente_ilegivel_de_ausente() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let base = std::env::temp_dir().join(format!("jim_inv_permissoes_{}", std::process::id()));
        let restrito = base.join("restrito");
        let sub = restrito.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("t.py"), b"existe").unwrap();
        let permissoes = fs::metadata(&restrito).unwrap().permissions();
        fs::set_permissions(&restrito, fs::Permissions::from_mode(0o600)).unwrap();
        let acesso = fs::symlink_metadata(&sub);
        let resultado = inventariar(base.to_str().unwrap(), &["restrito/sub/t.py".into()]);
        fs::set_permissions(&restrito, permissoes).unwrap();
        fs::remove_dir_all(&base).unwrap();

        // Executores privilegiados podem atravessar diretórios sem bit de busca.
        if let Err(e) = acesso {
            assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
            assert_eq!(resultado, Err("J-C7: arquivo ilegível: 'restrito/sub/t.py'".into()));
        }
        assert_eq!(erro_de_caminho(io::Error::from_raw_os_error(libc::EACCES), "'a/b'"),
                   "J-C7: arquivo ilegível: 'a/b'");
        assert_eq!(erro_de_caminho(io::Error::from_raw_os_error(libc::ENOTDIR), "'a/b'"),
                   "J-C4: arquivo ausente: 'a/b'");
    }

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
    #[cfg(unix)]
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
