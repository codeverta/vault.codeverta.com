use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{Manager, State};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

const CHECK_TEXT: &[u8] = b"vault-key-check-v1";
const EXPORT_MAGIC: &str = "CODEVERTA_VAULT_BACKUP_V1";

struct VaultState {
    db_path: PathBuf,
    key: Mutex<Option<Zeroizing<[u8; 32]>>>,
}

#[derive(Serialize)]
struct Status {
    initialized: bool,
    unlocked: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct VaultItem {
    id: String,
    title: String,
    item_type: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    secret: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    favorite: bool,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Deserialize)]
struct ItemInput {
    id: Option<String>,
    title: String,
    item_type: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    secret: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    favorite: bool,
}

#[derive(Serialize, Deserialize)]
struct CipherBlob {
    nonce: String,
    ciphertext: String,
}

#[derive(Serialize, Deserialize)]
struct BackupFile {
    magic: String,
    salt: String,
    nonce: String,
    ciphertext: String,
}

fn db(state: &VaultState) -> Result<Connection, String> {
    Connection::open(&state.db_path).map_err(|e| e.to_string())
}

fn init_db(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;
        CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value BLOB NOT NULL);
        CREATE TABLE IF NOT EXISTS vault_items (id TEXT PRIMARY KEY, encrypted_data BLOB NOT NULL, updated_at INTEGER NOT NULL);")
        .map_err(|e| e.to_string())
}

fn random<const N: usize>() -> [u8; N] {
    let mut out = [0u8; N];
    OsRng.fill_bytes(&mut out);
    out
}

fn derive_key(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, String> {
    let params = Params::new(65_536, 3, 1, Some(32)).map_err(|e| e.to_string())?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(password.as_bytes(), salt, key.as_mut())
        .map_err(|e| e.to_string())?;
    Ok(key)
}

fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<CipherBlob, String> {
    let nonce = random::<12>();
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| "Kunci tidak valid")?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|_| "Enkripsi gagal")?;
    Ok(CipherBlob {
        nonce: B64.encode(nonce),
        ciphertext: B64.encode(ciphertext),
    })
}

fn decrypt(key: &[u8; 32], blob: &CipherBlob) -> Result<Vec<u8>, String> {
    let nonce = B64.decode(&blob.nonce).map_err(|_| "Data nonce rusak")?;
    let ciphertext = B64
        .decode(&blob.ciphertext)
        .map_err(|_| "Data terenkripsi rusak")?;
    if nonce.len() != 12 {
        return Err("Data nonce rusak".into());
    }
    Aes256Gcm::new_from_slice(key)
        .map_err(|_| "Kunci tidak valid")?
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| "Password salah atau data rusak".into())
}

fn active_key(state: &VaultState) -> Result<Zeroizing<[u8; 32]>, String> {
    state
        .key
        .lock()
        .map_err(|_| "State vault bermasalah")?
        .as_ref()
        .map(|k| Zeroizing::new(**k))
        .ok_or_else(|| "Vault terkunci".into())
}

fn metadata(conn: &Connection, name: &str) -> Result<Option<Vec<u8>>, String> {
    conn.query_row("SELECT value FROM metadata WHERE key=?1", [name], |r| {
        r.get(0)
    })
    .optional()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn vault_status(state: State<VaultState>) -> Result<Status, String> {
    let conn = db(&state)?;
    init_db(&conn)?;
    Ok(Status {
        initialized: metadata(&conn, "salt")?.is_some(),
        unlocked: state
            .key
            .lock()
            .map_err(|_| "State vault bermasalah")?
            .is_some(),
    })
}

#[tauri::command]
fn create_vault(password: String, state: State<VaultState>) -> Result<(), String> {
    if password.len() < 10 {
        return Err("Master password minimal 10 karakter".into());
    }
    let conn = db(&state)?;
    init_db(&conn)?;
    if metadata(&conn, "salt")?.is_some() {
        return Err("Vault sudah dibuat".into());
    }
    let salt = random::<16>();
    let key = derive_key(&password, &salt)?;
    let check = serde_json::to_vec(&encrypt(&key, CHECK_TEXT)?).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO metadata(key,value) VALUES('salt',?1)",
        [salt.as_slice()],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO metadata(key,value) VALUES('key_check',?1)",
        [check],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    *state.key.lock().map_err(|_| "State vault bermasalah")? = Some(key);
    Ok(())
}

#[tauri::command]
fn unlock_vault(mut password: String, state: State<VaultState>) -> Result<(), String> {
    let conn = db(&state)?;
    let salt = metadata(&conn, "salt")?.ok_or("Vault belum dibuat")?;
    let raw = metadata(&conn, "key_check")?.ok_or("Metadata vault tidak lengkap")?;
    let key = derive_key(&password, &salt)?;
    password.zeroize();
    let blob: CipherBlob = serde_json::from_slice(&raw).map_err(|_| "Metadata vault rusak")?;
    if decrypt(&key, &blob)?.as_slice() != CHECK_TEXT {
        return Err("Master password salah".into());
    }
    *state.key.lock().map_err(|_| "State vault bermasalah")? = Some(key);
    Ok(())
}

#[tauri::command]
fn lock_vault(state: State<VaultState>) -> Result<(), String> {
    *state.key.lock().map_err(|_| "State vault bermasalah")? = None;
    Ok(())
}

#[tauri::command]
fn list_items(state: State<VaultState>) -> Result<Vec<VaultItem>, String> {
    let key = active_key(&state)?;
    let conn = db(&state)?;
    let mut stmt = conn
        .prepare("SELECT encrypted_data FROM vault_items ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| r.get::<_, Vec<u8>>(0))
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        let raw = row.map_err(|e| e.to_string())?;
        let blob: CipherBlob = serde_json::from_slice(&raw).map_err(|_| "Item vault rusak")?;
        let plain = decrypt(&key, &blob)?;
        out.push(serde_json::from_slice(&plain).map_err(|_| "Isi item vault rusak")?)
    }
    Ok(out)
}

#[tauri::command]
fn save_item(item: ItemInput, state: State<VaultState>) -> Result<VaultItem, String> {
    if item.title.trim().is_empty() {
        return Err("Nama item wajib diisi".into());
    }
    let key = active_key(&state)?;
    let conn = db(&state)?;
    let now = chrono::Utc::now().timestamp();
    let id = item.id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let created = conn
        .query_row(
            "SELECT encrypted_data FROM vault_items WHERE id=?1",
            [&id],
            |r| r.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .and_then(|raw| serde_json::from_slice::<CipherBlob>(&raw).ok())
        .and_then(|b| decrypt(&key, &b).ok())
        .and_then(|p| serde_json::from_slice::<VaultItem>(&p).ok())
        .map(|x| x.created_at)
        .unwrap_or(now);
    let saved = VaultItem {
        id: id.clone(),
        title: item.title,
        item_type: item.item_type,
        username: item.username,
        secret: item.secret,
        notes: item.notes,
        tags: item.tags,
        favorite: item.favorite,
        created_at: created,
        updated_at: now,
    };
    let blob = serde_json::to_vec(&encrypt(
        &key,
        &serde_json::to_vec(&saved).map_err(|e| e.to_string())?,
    )?)
    .map_err(|e| e.to_string())?;
    conn.execute("INSERT INTO vault_items(id,encrypted_data,updated_at) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET encrypted_data=excluded.encrypted_data,updated_at=excluded.updated_at",params![id,blob,now]).map_err(|e|e.to_string())?;
    Ok(saved)
}

#[tauri::command]
fn delete_item(id: String, state: State<VaultState>) -> Result<(), String> {
    active_key(&state)?;
    db(&state)?
        .execute("DELETE FROM vault_items WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn export_vault(
    password: String,
    path: String,
    state: State<VaultState>,
) -> Result<String, String> {
    if password.len() < 6 {
        return Err("Password backup minimal 6 karakter".into());
    }
    let items = list_items(state.clone())?;
    let salt = random::<16>();
    let key = derive_key(&password, &salt)?;
    let blob = encrypt(
        &key,
        &serde_json::to_vec(&items).map_err(|e| e.to_string())?,
    )?;
    let backup = BackupFile {
        magic: EXPORT_MAGIC.into(),
        salt: B64.encode(salt),
        nonce: blob.nonce,
        ciphertext: blob.ciphertext,
    };
    let mut output = PathBuf::from(path);
    if output.extension().is_none() {
        output.set_extension("vault");
    }
    fs::write(
        &output,
        serde_json::to_vec_pretty(&backup).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(output.to_string_lossy().into_owned())
}

#[tauri::command]
fn import_vault(password: String, path: String, state: State<VaultState>) -> Result<usize, String> {
    let backup: BackupFile = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|_| "Format backup tidak valid")?;
    if backup.magic != EXPORT_MAGIC {
        return Err("File ini bukan backup Vault yang valid".into());
    }
    let salt = B64.decode(backup.salt).map_err(|_| "Salt backup rusak")?;
    let key = derive_key(&password, &salt)?;
    let plain = decrypt(
        &key,
        &CipherBlob {
            nonce: backup.nonce,
            ciphertext: backup.ciphertext,
        },
    )?;
    let mut items: Vec<VaultItem> =
        serde_json::from_slice(&plain).map_err(|_| "Isi backup tidak valid")?;
    let vault_key = active_key(&state)?;
    let mut conn = db(&state)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for item in &mut items {
        if tx
            .query_row("SELECT 1 FROM vault_items WHERE id=?1", [&item.id], |r| {
                r.get::<_, i32>(0)
            })
            .optional()
            .map_err(|e| e.to_string())?
            .is_some()
        {
            item.id = Uuid::new_v4().to_string()
        }
        let raw = serde_json::to_vec(&encrypt(
            &vault_key,
            &serde_json::to_vec(item).map_err(|e| e.to_string())?,
        )?)
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO vault_items(id,encrypted_data,updated_at) VALUES(?1,?2,?3)",
            params![item.id, raw, item.updated_at],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(items.len())
}

#[tauri::command]
fn change_master_password(
    mut current_password: String,
    new_password: String,
    state: State<VaultState>,
) -> Result<(), String> {
    if new_password.len() < 10 {
        return Err("Master password baru minimal 10 karakter".into());
    }
    let mut conn = db(&state)?;
    let salt = metadata(&conn, "salt")?.ok_or("Vault belum dibuat")?;
    let old_key = derive_key(&current_password, &salt)?;
    current_password.zeroize();
    let check: CipherBlob =
        serde_json::from_slice(&metadata(&conn, "key_check")?.ok_or("Metadata tidak lengkap")?)
            .map_err(|_| "Metadata rusak")?;
    decrypt(&old_key, &check).map_err(|_| "Master password saat ini salah")?;
    let new_salt = random::<16>();
    let new_key = derive_key(&new_password, &new_salt)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let rows = {
        let mut stmt = tx
            .prepare("SELECT id,encrypted_data,updated_at FROM vault_items")
            .map_err(|e| e.to_string())?;
        let mapped = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    for (id, raw, updated) in rows {
        let blob: CipherBlob = serde_json::from_slice(&raw).map_err(|_| "Item rusak")?;
        let plain = decrypt(&old_key, &blob)?;
        let encrypted =
            serde_json::to_vec(&encrypt(&new_key, &plain)?).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE vault_items SET encrypted_data=?1,updated_at=?2 WHERE id=?3",
            params![encrypted, updated, id],
        )
        .map_err(|e| e.to_string())?;
    }
    let new_check =
        serde_json::to_vec(&encrypt(&new_key, CHECK_TEXT)?).map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE metadata SET value=?1 WHERE key='salt'",
        [new_salt.as_slice()],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE metadata SET value=?1 WHERE key='key_check'",
        [new_check],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    *state.key.lock().map_err(|_| "State vault bermasalah")? = Some(new_key);
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            fs::create_dir_all(&dir)?;
            let db_path = dir.join("vault.db");
            let conn = Connection::open(&db_path)?;
            init_db(&conn).map_err(std::io::Error::other)?;
            app.manage(VaultState {
                db_path,
                key: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            vault_status,
            create_vault,
            unlock_vault,
            lock_vault,
            list_items,
            save_item,
            delete_item,
            export_vault,
            import_vault,
            change_master_password
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vault")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encryption_round_trip_and_random_nonce() {
        let key = derive_key("a-strong-test-password", b"0123456789abcdef").unwrap();
        let first = encrypt(&key, b"super secret").unwrap();
        let second = encrypt(&key, b"super secret").unwrap();
        assert_ne!(first.nonce, second.nonce);
        assert_ne!(first.ciphertext, second.ciphertext);
        assert_eq!(decrypt(&key, &first).unwrap(), b"super secret");
    }

    #[test]
    fn tampering_is_rejected() {
        let key = derive_key("a-strong-test-password", b"0123456789abcdef").unwrap();
        let mut blob = encrypt(&key, b"super secret").unwrap();
        blob.ciphertext.push('A');
        assert!(decrypt(&key, &blob).is_err());
    }
}
