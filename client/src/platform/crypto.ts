/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

// Import private key for decryption (RSA-OAEP)
export async function importRsaPrivateKeyForDecrypt(pem: string): Promise<CryptoKey> {
  const b64 = pem.replace(/-----.*?-----/g, '').replace(/\s+/g, '');
  const der = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
  return window.crypto.subtle.importKey(
    'pkcs8',
    der.buffer,
    { name: 'RSA-OAEP', hash: 'SHA-256' },
    false,
    ['decrypt']
  );
}

// RSA encryption/signature helpers using Web Crypto API
export async function encryptWithPublicKey(pem: string, plaintext: string): Promise<string> {
  // Convert PEM to CryptoKey
  const key = await importRsaPublicKey(pem);
  const enc = new TextEncoder();
  const data = enc.encode(plaintext);
  const ciphertext = await window.crypto.subtle.encrypt({ name: 'RSA-OAEP' }, key, data);
  return btoa(String.fromCharCode(...new Uint8Array(ciphertext)));
}

export async function signContent(pem: string, plaintext: string): Promise<string> {
  // Convert PEM to CryptoKey
  const key = await importRsaPrivateKey(pem);
  const enc = new TextEncoder();
  const data = enc.encode(plaintext);
  const signature = await window.crypto.subtle.sign({ name: 'RSASSA-PKCS1-v1_5' }, key, data);
  return btoa(String.fromCharCode(...new Uint8Array(signature)));
}

export async function importRsaPublicKey(pem: string): Promise<CryptoKey> {
  // Remove header/footer and newlines
  const b64 = pem.replace(/-----.*?-----/g, '').replace(/\s+/g, '');
  const der = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
  return window.crypto.subtle.importKey(
    'spki',
    der.buffer,
    { name: 'RSA-OAEP', hash: 'SHA-256' },
    false,
    ['encrypt']
  );
}

export async function importRsaPrivateKey(pem: string): Promise<CryptoKey> {
  // Remove header/footer and newlines
  const b64 = pem.replace(/-----.*?-----/g, '').replace(/\s+/g, '');
  const der = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
  return window.crypto.subtle.importKey(
    'pkcs8',
    der.buffer,
    { name: 'RSASSA-PKCS1-v1_5', hash: 'SHA-256' },
    false,
    ['sign']
  );
}
