use base64::{encode, decode};

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedSet, UnorderedMap};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault, Balance, Promise};
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::json_types::U128;

use crate::user_account::*;

mod user_account;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct PassManager {
    pub owner_id: AccountId,
    pub accounts_per_user: UnorderedMap<AccountId, UnorderedSet<UserAccountId>>,
    pub accounts_by_id: UnorderedMap<UserAccountId, UserAccount>,
    pub account_id_counter: UserAccountId
}

#[near_bindgen]
impl PassManager{
    #[init]
    pub fn new(owner_id: AccountId) -> Self{
        Self { 
            owner_id, 
            accounts_per_user: UnorderedMap::new("apu".try_to_vec().unwrap()),
            accounts_by_id: UnorderedMap::new("abi".try_to_vec().unwrap()),
            account_id_counter: 0
        }
    }

    #[payable]
    pub fn add_account(&mut self, user_id: AccountId, website: String, mut username: String, mut password: String){

        //Assert deposit is attached = full access key provided
        assert!(env::attached_deposit() >= 1, "Required attached deposit of at least 1 yoctoNEAR");
        let init_storage_used = env::storage_usage();

        let mut id: u128 = 0;

        //Check if account is new or needs to be updated
        if let Some(account) = self.get_one_account(user_id.clone(), website.clone()){
            id = account.id;
        }else{
            self.account_id_counter += 1;
            id = self.account_id_counter; 
        }

        //Base64 encode
        username = encode(username);
        password = encode(password);

        let account = UserAccount { id, user_id: user_id.clone(), website, username, password };

        self.accounts_by_id.insert(&account.id, &account);
        self.add_account_to_user(&user_id, &account.id); 

        let storage_used = env::storage_usage() - init_storage_used;
        let required_cost = env::storage_byte_cost() * Balance::from(storage_used);
        let attached_deposit = env::attached_deposit();

        //Remove previously added credentials and panic if attached deposit < required cost
        if required_cost > attached_deposit {
            self.accounts_by_id.remove(&account.id);
            self.remove_account_from_user(&user_id, &account.id);            
            env::panic_str(&format!("Must attach {} yoctoNEAR to cover storage", required_cost));
        }
        
        let refund = attached_deposit - required_cost;

        //Refund remaining yocto if greater than 1
        if refund > 1 {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }
        
    }

    pub fn get_one_account(&self, user_id: AccountId, website: String) -> Option<UserAccount>{
        if let Some(acc_set) = self.accounts_per_user.get(&user_id){
            for acc in acc_set.iter(){
                let mut a = self.accounts_by_id.get(&acc).unwrap();
                if &a.website == &website{
                    decode_credentials(&mut a);
                    return Some(a);
                }
            }
        }        
        None
    }

    pub fn get_accounts_per_user(&self, user_id: AccountId) -> Vec<UserAccount>{
        let acc_set = self.accounts_per_user.get(&user_id).expect("Invalid user");

        acc_set.iter().map(|x| {
            let mut acc = self.accounts_by_id.get(&x).unwrap();
            decode_credentials(&mut acc);
            acc            
        }).collect()

    }

    //Get all accounts without decrypting for testing
    pub fn get_all_accounts(&self) -> Vec<UserAccount>{
        self.accounts_by_id.iter().map(|(_k, v)| v).collect()        
    }

    pub fn get_users_count(&self) -> U128{
        U128(self.accounts_per_user.len() as u128)
    }

    pub fn remove_account(&mut self, user_id: AccountId, account_id: UserAccountId){
        let init_storage_used = env::storage_usage();
        
        let removed_from_user = self.remove_account_from_user(&user_id, &account_id);

        if !removed_from_user {
            env::panic_str("User account not found")
        }
        self.accounts_by_id.remove(&account_id);          

        let storage_released = init_storage_used - env::storage_usage();
        let refund = env::storage_byte_cost() * Balance::from(storage_released);

        //Refund for releasing storage if greater than 1 yocto
        if refund > 1 {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }
    }
}
