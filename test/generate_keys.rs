// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

use std::fs;

use secure_chat::crypto::RsaUtil;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure the keys directory exists
    fs::create_dir_all("keys")?;

    let server_names = ["server1", "server2", "server3"];
    for server_name in &server_names {
        println!("Generating keys for {server_name}");

        let rsa_util = RsaUtil::new()?;

        let private_key_pem = rsa_util.privkey_pem()?;
        let public_key_pem = rsa_util.pubkey_pem()?;
        // Get the base64url public key for config
        let pubkey_b64 = rsa_util.pubkey_base64url()? + "\n";

        fs::write(
            format!("keys/{server_name}_private_key.pem"),
            private_key_pem.as_bytes(),
        )?;

        fs::write(
            format!("keys/{server_name}_public_key.pem"),
            public_key_pem.as_bytes(),
        )?;

        fs::write(
            format!("keys/{server_name}_pubkey.txt"),
            pubkey_b64.as_bytes(),
        )?;

        println!(
            "    Generated keys for {server_name} (pubkey: {}...)",
            &pubkey_b64[..20]
        );
    }

    println!("All keys generated successfully!");
    Ok(())
}
