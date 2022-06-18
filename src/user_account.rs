use crate::*;
use near_sdk::{CryptoHash};

pub type UserAccountId = u128;


#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct UserAccount{
    pub id: UserAccountId,
    pub user_id: AccountId,
    pub website: String,
    pub username: String,
    pub password: String

}

pub(crate) fn hash_account_id(account_id: &AccountId) -> CryptoHash{
    let mut hash = CryptoHash::default();
    hash.copy_from_slice(&env::sha256(account_id.as_bytes()));
    hash
}

pub(crate) fn decode_credentials(account: &mut UserAccount){
    let decoded = match decode(&account.username){
        Ok(d) => d,
        Err(_) => env::panic_str("Cannot decode username")
    };
    let username = match std::str::from_utf8(&decoded){
        Ok(u) => u,
        Err(_) => env::panic_str("Not a string")
    };
    account.username = username.to_string();

    let decoded = match decode(&account.password){
        Ok(d) => d,
        Err(_) => env::panic_str("Cannot decode password")
    };
    let pass = match std::str::from_utf8(&decoded){
        Ok(p) => p,
        Err(_) => env::panic_str("Not a string")
    };
    account.password = pass.to_string();
}

impl PassManager{
    pub(crate)fn add_account_to_user(&mut self, user_id: &AccountId, account_id: &UserAccountId){
        let mut account_set = self.accounts_per_user.get(user_id).unwrap_or_else(||{
            UnorderedSet::new(hash_account_id(user_id).try_to_vec().unwrap())
        });

        account_set.insert(account_id);
        self.accounts_per_user.insert(user_id, &account_set);
    }

    pub(crate) fn remove_account_from_user(&mut self, user_id: &AccountId, account_id: &UserAccountId) -> bool{
        let mut account_set = self.accounts_per_user.get(user_id).expect("Invalid User");

        let removed = account_set.remove(account_id);

        if account_set.is_empty(){
            self.accounts_per_user.remove(user_id);
        }else{
            self.accounts_per_user.insert(user_id, &account_set);
        }
        removed
    }    
}