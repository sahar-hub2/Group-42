/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import { useState } from 'react';
import { addUser, User } from '../platform/db';
import { encryptPrivateKey } from '../platform/privkey_encrypt';
import { v4 as uuidv4 } from 'uuid';

// PBKDF2 password hashing using Web Crypto API
async function pbkdf2Hash(password: string, salt: string): Promise<string> {
  const enc = new TextEncoder();
  const key = await window.crypto.subtle.importKey(
    'raw',
    enc.encode(password),
    { name: 'PBKDF2' },
    false,
    ['deriveBits']
  );
  const derived = await window.crypto.subtle.deriveBits(
    {
      name: 'PBKDF2',
      salt: enc.encode(salt),
      iterations: 100_000,
      hash: 'SHA-256',
    },
    key,
    256
  );
  return Array.from(new Uint8Array(derived))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

interface RegisterScreenProps {
  onRegisterComplete: () => void;
}

function RegisterScreen({ onRegisterComplete }: RegisterScreenProps) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  // Password requirement checks
  const passwordChecks = {
    length: password.length >= 8,
    number: /\d/.test(password),
    special: /[!@#$%^&*(),.?":{}|<>]/.test(password),
  };

  const handleRegister = async () => {
    setError(null);
    setSuccess(null);
    if (!username.trim() || !password.trim()) {
      setError('Username and password are required');
      return;
    }
    if (!passwordChecks.length || !passwordChecks.number || !passwordChecks.special) {
      setError('Password does not meet requirements');
      return;
    }
    if (password !== confirmPassword) {
      setError('Passwords do not match');
      return;
    }

    // Generate user_id and hash password using PBKDF2
    const user_id = uuidv4();
    const salt = uuidv4();
    const passwordHash = await pbkdf2Hash(password, salt);

    // Generate RSA keypair (4096 bits)
    let pubkeyPem = '';
    let privkeyPem = '';
    try {
      const keyPair = await window.crypto.subtle.generateKey(
        {
          name: 'RSA-OAEP',
          modulusLength: 4096,
          publicExponent: new Uint8Array([1, 0, 1]),
          hash: 'SHA-256',
        },
        true,
        ['encrypt', 'decrypt']
      );
      // Export public key to PEM
      const spki = await window.crypto.subtle.exportKey('spki', keyPair.publicKey);
      const b64pub = btoa(String.fromCharCode(...new Uint8Array(spki)));
      pubkeyPem = `-----BEGIN PUBLIC KEY-----\n${b64pub.match(/.{1,64}/g)?.join('\n') || b64pub}\n-----END PUBLIC KEY-----`;
      // Export private key to PEM
      const pkcs8 = await window.crypto.subtle.exportKey('pkcs8', keyPair.privateKey);
      const b64priv = btoa(String.fromCharCode(...new Uint8Array(pkcs8)));
      privkeyPem = `-----BEGIN PRIVATE KEY-----\n${b64priv.match(/.{1,64}/g)?.join('\n') || b64priv}\n-----END PRIVATE KEY-----`;
    } catch (e) {
      setError('Failed to generate keypair: ' + e);
      return;
    }
    // Encrypt private key with password using AES-GCM
    const privkey_store = await encryptPrivateKey(privkeyPem, passwordHash);
    const user: User = {
      user_id,
      pubkey: pubkeyPem,
      privkey_store,
      password: passwordHash,
      salt,
      meta: { display_name: username },
      version: 1,
    };
    try {
      await addUser(user);
      setSuccess(`Registered successfully as ${username}`);
      setTimeout(() => {
        setSuccess(null);
        onRegisterComplete();
      }, 1200);
    } catch (e: unknown) {
      setError('Registration failed: ' + (e instanceof Error ? e.message : String(e)));
    }
  };

  return (
    <div className="login-container">
      <h1>Create Account</h1>
      {error && <div style={{ color: 'red', marginBottom: '0.5rem' }}>{error}</div>}
      {success && <div style={{ color: 'green', marginBottom: '0.5rem' }}>{success}</div>}
      <input
        type="text"
        placeholder="Username"
        value={username}
        onChange={(e) => setUsername(e.target.value)}
      />
      <input
        type="password"
        placeholder="Password"
        value={password}
        onChange={(e) => setPassword(e.target.value)}
      />

      {/* Password requirements list */}
      <ul style={{ fontSize: '0.9rem', margin: '0.25rem 0', paddingLeft: '1.25rem' }}>
        <li style={{ color: passwordChecks.length ? '#10b981' : '#ef4444' }}>
          Minimum 8 characters
        </li>
        <li style={{ color: passwordChecks.number ? '#10b981' : '#ef4444' }}>At least 1 number</li>
        <li style={{ color: passwordChecks.special ? '#10b981' : '#ef4444' }}>
          At least 1 special character
        </li>
      </ul>

      <input
        type="password"
        placeholder="Confirm Password"
        value={confirmPassword}
        onChange={(e) => setConfirmPassword(e.target.value)}
      />

      <button className="register-button" onClick={handleRegister}>
        Register
      </button>
      <button className="login-button" onClick={onRegisterComplete}>
        Back to Login
      </button>
    </div>
  );
}

export default RegisterScreen;
