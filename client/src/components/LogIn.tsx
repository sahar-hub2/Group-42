/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import { useState } from 'react';
import { listUsers, User } from '../platform/db';

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

interface LoginScreenProps {
  onLogin: (user: User) => void;
  onRegister: () => void;
}

function LogIn({ onLogin, onRegister }: LoginScreenProps) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);

  const handleLogin = async () => {
    setError(null);
    if (!username.trim() || !password.trim()) {
      setError('Username and password are required');
      return;
    }
    // Find user by display_name in meta
    const users = await listUsers();
    const user = users.find(
      (u) => u.meta && u.meta.display_name && u.meta.display_name === username
    );
    if (!user) {
      setError('User not found');
      return;
    }
    // Use PBKDF2 to verify password
    const hashToCheck = await pbkdf2Hash(password, user.salt);
    if (hashToCheck !== user.password) {
      setError('Incorrect password');
      return;
    }
    onLogin(user);
  };

  return (
    <div className="login-container">
      <h1>Welcome</h1>
      {error && <div style={{ color: 'red', marginBottom: '0.5rem' }}>{error}</div>}
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
      <button className="login-button" onClick={handleLogin}>
        Log In
      </button>
      <button className="register-button" onClick={onRegister}>
        Register
      </button>
    </div>
  );
}

export default LogIn;
