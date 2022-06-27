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
    /*
    pub fn get_all_accounts(&self) -> Vec<UserAccount>{
        self.accounts_by_id.iter().map(|(_k, v)| v).collect()        
    }*/

    pub fn get_users_count(&self) -> U128{
        U128(self.accounts_per_user.len() as u128)
    }

    pub fn remove_account(&mut self, user_id: AccountId, account_id: UserAccountId){
        let init_storage_used = env::storage_usage();
        
        let removed_from_user = self.remove_account_from_user(&user_id, &account_id);

        if !removed_from_user {
            env::panic_str("Account not found")
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


#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, VMContext};

    fn get_context(needs_deposit: bool) -> VMContext {
        if needs_deposit {
            VMContextBuilder::new()
            .signer_account_id("milos21.testnet".parse().unwrap())
            .attached_deposit(10000000000000000000000)
            .is_view(false)
            .build()
        }else{
            VMContextBuilder::new()
            .signer_account_id("milos21.testnet".parse().unwrap())
            .is_view(false)
            .build()
        }
    }

    #[test]
    fn add_account_success_test() {
        let context = get_context(true);
        testing_env!(context);
        let mut pass_manager = PassManager::new("milos21.testnet".parse().unwrap());
        assert_eq!((), pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "instagram".to_string(), "user1".to_string(), "pass1".to_string()));
    }

    #[test]
    #[should_panic(expected = r#"Required attached deposit of at least 1 yoctoNEAR"#)]
    fn add_account_panic_test(){
        //set up context without deposit, test needs to panic if no deposit is attached
        let context = get_context(false);
        testing_env!(context);
        let mut pass_manager = PassManager::new("milos21.testnet".parse().unwrap());
        pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "instagram".to_string(), "user1".to_string(), "pass1".to_string());    
    }

    #[test]
    fn get_one_account_find_some_test(){
        let context = get_context(true);
        testing_env!(context);
        let mut pass_manager = PassManager::new("milos21.testnet".parse().unwrap());
        pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "instagram".to_string(), "user1".to_string(), "pass1".to_string());   
        assert_eq!(
            Some(UserAccount{
                id: 1, 
                user_id: "1.milos21.testnet".parse().unwrap(), 
                website: "instagram".to_string(), 
                username: "user1".to_string(),
                password: "pass1".to_string()}), 
            pass_manager.get_one_account("1.milos21.testnet".parse().unwrap(), "instagram".parse().unwrap()));
    }

    #[test]
    fn get_one_account_invalid_user_test(){
        let context = get_context(true);
        testing_env!(context);
        let mut pass_manager = PassManager::new("milos21.testnet".parse().unwrap());
        pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "instagram".to_string(), "user1".to_string(), "pass1".to_string());   
        
        //Non existent user so it should return None
        assert_eq!(
            None, 
            pass_manager.get_one_account("2.milos21.testnet".parse().unwrap(), "instagram".parse().unwrap()));
    }

    #[test]
    fn get_one_account_invalid_website_test(){
        let context = get_context(true);
        testing_env!(context);
        let mut pass_manager = PassManager::new("milos21.testnet".parse().unwrap());
        pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "instagram".to_string(), "user1".to_string(), "pass1".to_string());   
        
        //Non existent website so it should return None
        assert_eq!(
            None, 
            pass_manager.get_one_account("1.milos21.testnet".parse().unwrap(), "facebook".parse().unwrap()));
    }

    #[test]
    fn get_accounts_per_user_find_some_test(){
        let context = get_context(true);
        testing_env!(context);
        let mut pass_manager = PassManager::new("milos21.testnet".parse().unwrap());
        pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "instagram".to_string(), "user1".to_string(), "pass1".to_string());   
        pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "facebook".to_string(), "user2".to_string(), "pass2".to_string());   
        pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "reddit".to_string(), "user3".to_string(), "pass3".to_string());
        pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "twitter".to_string(), "user4".to_string(), "pass4".to_string());   
        let mut account = UserAccount{
                                        id: 1,  
                                        user_id: "1.milos21.testnet".parse().unwrap(), 
                                        website: "instagram".to_string(), 
                                        username: "user1".to_string(),
                                        password: "pass1".to_string()
                                    };
        let mut acc_vec: Vec<UserAccount> = vec![];
        acc_vec.push(account.clone());

        account.id = 2;
        account.website = "facebook".to_string();
        account.username = "user2".to_string();
        account.password = "pass2".to_string();
        acc_vec.push(account.clone());

        account.id = 3;
        account.website = "reddit".to_string();
        account.username = "user3".to_string();
        account.password = "pass3".to_string();
        acc_vec.push(account.clone());

        account.id = 4;
        account.website = "twitter".to_string();
        account.username = "user4".to_string();
        account.password = "pass4".to_string();
        acc_vec.push(account);

        
        assert_eq!(
            acc_vec, 
            pass_manager.get_accounts_per_user("1.milos21.testnet".parse().unwrap()));
    }

    #[test]
    #[should_panic(expected = r#"Invalid user"#)]
    fn get_accounts_per_user_non_existent_user_test(){
        let context = get_context(true);
        testing_env!(context);
        let mut pass_manager = PassManager::new("milos21.testnet".parse().unwrap());
        pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "instagram".to_string(), "user1".to_string(), "pass1".to_string());   
                
        //Non existent user, test should panic
        pass_manager.get_accounts_per_user("2.milos21.testnet".parse().unwrap());
    }

    #[test]
    fn get_users_count_test(){
        let context = get_context(true);
        testing_env!(context);
        let mut pass_manager = PassManager::new("milos21.testnet".parse().unwrap());
        pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "instagram".to_string(), "user1".to_string(), "pass1".to_string());   
                
        assert_eq!(U128(1), pass_manager.get_users_count());
    }

    #[test]
    fn remove_account_sucess_test(){
        let context = get_context(true);
        testing_env!(context);
        let mut pass_manager = PassManager::new("milos21.testnet".parse().unwrap());
        pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "instagram".to_string(), "user1".to_string(), "pass1".to_string());   
                
        assert_eq!((), pass_manager.remove_account("1.milos21.testnet".parse().unwrap(), 1));
    }

    #[test]
    #[should_panic(expected = r#"Invalid User"#)]
    fn remove_account_invalid_user_test(){
        let context = get_context(true);
        testing_env!(context);
        let mut pass_manager = PassManager::new("milos21.testnet".parse().unwrap());
        pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "instagram".to_string(), "user1".to_string(), "pass1".to_string());   
                
        //Non existent user, test should panic
        pass_manager.remove_account("2.milos21.testnet".parse().unwrap(), 1);
    }

    #[test]
    #[should_panic(expected = r#"Account not found"#)]
    fn remove_account_invalid_account_test(){
        let context = get_context(true);
        testing_env!(context);
        let mut pass_manager = PassManager::new("milos21.testnet".parse().unwrap());
        pass_manager.add_account("1.milos21.testnet".parse().unwrap(), "instagram".to_string(), "user1".to_string(), "pass1".to_string());   
                
        //Non existent account, test should panic
        pass_manager.remove_account("1.milos21.testnet".parse().unwrap(), 2);
    }
}
