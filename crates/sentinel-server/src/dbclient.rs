//! Cliente de linha de comando para servidores MySQL/MariaDB (do Sentinel
//! ou já existentes no PC): SQL, criar banco, listar, backup e restauração.
//! A senha vai pela variável `MYSQL_PWD`, nunca na linha de comando.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct DbClient {
    /// Pasta com `mariadb.exe`/`mariadb-dump.exe` ou `mysql.exe`/`mysqldump.exe`.
    pub bin_dir: PathBuf,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
}

/// Nome válido para banco (letras, números e `_`), até 64 caracteres.
pub fn valid_db(name: &str) -> bool {
    !name.is_empty() && name.len() <= 64 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// A pasta tem um cliente de linha de comando utilizável?
pub fn has_client(dir: &Path) -> bool {
    dir.join("mariadb.exe").is_file() || dir.join("mysql.exe").is_file()
}

impl DbClient {
    /// `mariadb.exe` se existir; senão o nome do MySQL (`mysql.exe`).
    fn tool(&self, maria: &str, mysql: &str) -> PathBuf {
        let m = self.bin_dir.join(format!("{maria}.exe"));
        if m.is_file() { m } else { self.bin_dir.join(format!("{mysql}.exe")) }
    }

    fn command(&self, maria: &str, mysql: &str) -> Command {
        let tool = self.tool(maria, mysql);
        let is_maria = tool.file_name().is_some_and(|n| n.to_string_lossy().starts_with("mariadb"));
        let mut c = Command::new(&tool);
        #[cfg(windows)]
        c.creation_flags(0x0800_0000);
        if is_maria {
            // Plugins de autenticação (ex.: caching_sha2_password do MySQL 8)
            // ficam em lib\plugin da instalação.
            let plugins = self.bin_dir.parent().map(|d| d.join("lib").join("plugin"));
            if let Some(p) = plugins.filter(|p| p.is_dir()) {
                c.arg(format!("--plugin-dir={}", p.display()));
            }
            // Conexão local: sem verificar certificado (evita aviso e falha
            // em servidores sem SSL configurado).
            c.arg("--skip-ssl-verify-server-cert");
        }
        c.args(["-u", &self.user, "-h", &self.host, "-P", &self.port.to_string()]);
        c.env("MYSQL_PWD", &self.password);
        c.stdin(Stdio::null());
        c
    }

    fn fail(out: &std::process::Output) -> String {
        // Avisos do cliente (WARNING: …) não são o erro em si.
        let stderr = String::from_utf8_lossy(&out.stderr);
        let text = stderr
            .lines()
            .filter(|l| !l.trim_start().starts_with("WARNING"))
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        if text.is_empty() { format!("código {:?}", out.status.code()) } else { text }
    }

    /// Roda SQL e devolve a saída (sem cabeçalhos).
    pub async fn sql(&self, statement: &str) -> Result<String, String> {
        let out = self
            .command("mariadb", "mysql")
            .args(["-N", "-B", "-e", statement])
            .output()
            .await
            .map_err(|e| format!("não foi possível rodar o cliente do banco: {e}"))?;
        if !out.status.success() {
            return Err(Self::fail(&out));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    /// Versão do servidor (também serve de teste de conexão).
    pub async fn version(&self) -> Result<String, String> {
        Ok(self.sql("SELECT VERSION()").await?.trim().to_string())
    }

    pub async fn create_database(&self, name: &str) -> Result<(), String> {
        if !valid_db(name) {
            return Err(format!("nome de banco inválido: {name}"));
        }
        self.sql(&format!("CREATE DATABASE IF NOT EXISTS `{name}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci"))
            .await
            .map(|_| ())
    }

    pub async fn databases(&self) -> Result<Vec<String>, String> {
        Ok(self
            .sql("SHOW DATABASES")
            .await?
            .lines()
            .map(str::trim)
            .filter(|d| !["information_schema", "mysql", "performance_schema", "sys", ""].contains(d))
            .map(String::from)
            .collect())
    }

    /// Exporta um banco para `.sql`.
    pub async fn dump(&self, db: &str, file: &Path) -> Result<(), String> {
        if !valid_db(db) {
            return Err(format!("nome de banco inválido: {db}"));
        }
        if let Some(p) = file.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let out_file = std::fs::File::create(file).map_err(|e| e.to_string())?;
        // `spawn` + espera: `.output()` capturaria a saída em vez de gravar no arquivo.
        let out = self
            .command("mariadb-dump", "mysqldump")
            .args(["--single-transaction", "--routines", "--triggers", "--default-character-set=utf8mb4", db])
            .stdout(Stdio::from(out_file))
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
            .wait_with_output()
            .await
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            let _ = std::fs::remove_file(file);
            return Err(Self::fail(&out));
        }
        Ok(())
    }

    /// Importa um `.sql` num banco (cria se não existir).
    pub async fn restore(&self, db: &str, file: &Path) -> Result<(), String> {
        self.create_database(db).await?;
        let input = std::fs::File::open(file).map_err(|e| e.to_string())?;
        let out = self
            .command("mariadb", "mysql")
            .args(["--default-character-set=utf8mb4", db])
            .stdin(Stdio::from(input))
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(Self::fail(&out));
        }
        Ok(())
    }
}
