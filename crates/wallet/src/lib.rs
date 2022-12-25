pub mod wallet;


#[cfg(test)]

use wallet::WalletAccount;

    #[test]
    fn generate_new_wallet() {
        let wallet = WalletAccount::new();
        let prefix = wallet.addresses[&1].clone() ;
        assert!(wallet.total_balances[&prefix][&"VRRB".to_string()] == 1000);
    }

    #[test]
    fn generate_new_wallet_addrs() {
        let mut wallet = WalletAccount::new();
        let prefix = wallet.addresses[&1].clone() ;
        assert!(wallet.total_balances[&prefix][&"VRRB".to_string()] == 1000);
        
        wallet.get_new_addresses(5);
        assert!(wallet.addresses.len() == 5);

        let addrs = wallet.get_wallet_addresses();
        
        assert!(addrs.len() == 5);
        assert!(addrs[&1] == wallet.addresses[&1]);
    }

    #[test]
    fn check_claims() {
        let wallet = WalletAccount::new();
        let prefix = wallet.addresses[&1].clone() ;
        assert!(wallet.total_balances[&prefix][&"VRRB".to_string()] == 1000);

        dbg!("NUM CLAIMS: {}", wallet.claims.len());
        
        assert!(wallet.claims.len() == 0);
    }

    #[test]
    fn sign_and_verify() {
        let wallet = WalletAccount::new();
        let prefix = wallet.addresses[&1].clone() ;
        assert!(wallet.total_balances[&prefix][&"VRRB".to_string()] == 1000);

        let msg = "huh??";

        let sig = wallet.sign(&msg).unwrap();

        WalletAccount::verify(msg.to_string(), sig, wallet.pubkey);
    }




    


