/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

// AES-GCM encryption for private key storage
// Usage: encryptPrivateKey(plain: string, passwordHash: string): Promise<string>

export async function encryptPrivateKey(plain: string, passwordHash: string): Promise<string> {
  const enc = new TextEncoder();
  // Derive a key from the password hash (hex string)
  const pwBytes = Uint8Array.from(passwordHash.match(/.{1,2}/g)!.map((b) => parseInt(b, 16)));
  const key = await window.crypto.subtle.importKey('raw', pwBytes, { name: 'AES-GCM' }, false, [
    'encrypt',
  ]);
  const iv = window.crypto.getRandomValues(new Uint8Array(12));
  const ciphertext = await window.crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    key,
    enc.encode(plain)
  );
  // Store as base64: iv:ciphertext
  return (
    btoa(String.fromCharCode(...iv)) +
    ':' +
    btoa(String.fromCharCode(...new Uint8Array(ciphertext)))
  );
}
