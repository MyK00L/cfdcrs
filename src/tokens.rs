use crate::serenity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::ops::Deref;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use uuid::Uuid;

const FILE_NAME: &str = "tokens.ron";

type Error = Box<dyn std::error::Error + Send + Sync>;
pub type TokenKey = u128;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenData {
    pub roles: Vec<serenity::RoleId>,
    pub limit: u32,
    pub expiration: SystemTime,
}

pub type DbType = Arc<RwLock<HashMap<TokenKey, TokenData>>>;

pub async fn create_db() -> Result<(), Error> {
    let db = HashMap::<TokenKey, TokenData>::new();
    let data = ron::to_string(&db)?;
    fs::write(FILE_NAME, &data)?;
    Ok(())
}

pub async fn load_db() -> Result<DbType, Error> {
    let data: HashMap<TokenKey, TokenData> = ron::from_str(&fs::read_to_string(FILE_NAME)?)?;
    Ok(Arc::new(RwLock::new(data)))
}

pub async fn _get_token(db_lock: DbType, key: TokenKey) -> Result<TokenData, Error> {
    let db = db_lock.read().await;
    let ans = db.get(&key).ok_or("token not found")?.clone();
    Ok(ans)
}

pub async fn use_token(db_lock: DbType, key: TokenKey) -> Result<Vec<serenity::RoleId>, Error> {
    let mut ans = {
        let db = db_lock.read().await;
        db.get(&key).ok_or("token not found")?.clone()
    };
    if SystemTime::now() > ans.expiration {
        ans.limit = 0;
    }
    let res = if ans.limit == 0 {
        Err("token expired".into())
    } else {
        ans.limit -= 1;
        Ok(ans.roles.clone())
    };
    if ans.limit == 0 {
        rem_token(db_lock.clone(), key).await?;
    } else {
        set_token(db_lock.clone(), key, ans.clone()).await?;
    }
    res
}

pub async fn set_token(db_lock: DbType, key: TokenKey, data: TokenData) -> Result<(), Error> {
    let mut db = db_lock.write().await;
    db.insert(key, data);
    let data = ron::to_string(&db.deref())?;
    fs::write(FILE_NAME, &data)?;
    Ok(())
}

pub async fn add_token(db_lock: DbType, data: TokenData) -> Result<TokenKey, Error> {
    let key: u128 = Uuid::new_v4().as_u128();
    set_token(db_lock, key, data).await?;
    Ok(key)
}

pub async fn rem_token(db_lock: DbType, key: TokenKey) -> Result<(), Error> {
    let mut db = db_lock.write().await;
    db.remove(&key);
    let data = ron::to_string(&db.deref())?;
    fs::write(FILE_NAME, &data)?;
    Ok(())
}
