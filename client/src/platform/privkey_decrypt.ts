/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

// AES-GCM decryption for private key storage
// Usage: decryptPrivateKey(cipher: string, passwordHash: string): Promise<string>

export async function decryptPrivateKey(cipher: string, passwordHash: string): Promise<string> {
  const [ivB64, ctB64] = cipher.split(':');
  if (!ivB64 || !ctB64) throw new Error('Invalid encrypted private key format');
  const iv = Uint8Array.from(atob(ivB64), (c) => c.charCodeAt(0));
  const ciphertext = Uint8Array.from(atob(ctB64), (c) => c.charCodeAt(0));
  // Derive a key from the password hash (hex string)
  const pwBytes = Uint8Array.from(passwordHash.match(/.{1,2}/g)!.map((b) => parseInt(b, 16)));
  const key = await window.crypto.subtle.importKey('raw', pwBytes, { name: 'AES-GCM' }, false, [
    'decrypt',
  ]);
  const plain = await window.crypto.subtle.decrypt({ name: 'AES-GCM', iv }, key, ciphertext);
  return new TextDecoder().decode(plain);
}
