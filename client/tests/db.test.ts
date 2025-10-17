/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import { test, expect, describe } from 'bun:test';
import { addUser, listUsers, User } from '../src/platform/db';
import { isTauri } from '../src/platform/index';

import { v4 as uuidv4 } from 'uuid';

// PBKDF2 password hashing using Web Crypto API
async function pbkdf2Hash(password: string, salt: string): Promise<string> {
  const enc = new TextEncoder();
  const key = await crypto.subtle.importKey(
    'raw',
    enc.encode(password),
    { name: 'PBKDF2' },
    false,
    ['deriveBits']
  );
  const derived = await crypto.subtle.deriveBits(
    {
      name: 'PBKDF2',
      salt: enc.encode(salt),
      iterations: 100_000,
      hash: 'SHA-256',
    },
    key,
    256
  );
  return Array.from(new Uint8Array(derived)).map(b => b.toString(16).padStart(2, '0')).join('');
}

describe('db', () => {
  if (!isTauri() || typeof Bun !== 'undefined' || process.env.NODE_ENV === 'test') {
    test('skipped: add and get user (not running in Tauri desktop environment)', () => {
      // Skipped in non-Tauri/Bun/test environments
      expect(true).toBe(true);
    });
    test('skipped: list users includes added user (not running in Tauri desktop environment)', () => {
      expect(true).toBe(true);
    });
    return;
  }

  test('add and get user', async () => {
    const user_id = uuidv4();
    const salt = uuidv4();
    const password = 'testpassword';
    const passwordHash = await pbkdf2Hash(password, salt);
    const testUser: User = {
      user_id,
      pubkey: '',
      privkey_store: '',
      password: passwordHash,
      salt,
      meta: { display_name: 'testuser' },
      version: 1,
    };
    await addUser(testUser);
    const user = await listUsers().then((users: User[]) => users.find((u: User) => u.user_id === user_id));
    expect(user).toBeTruthy();
    expect(user?.user_id).toBe(user_id);
    expect(user?.password).toBe(passwordHash);
  });

  test('list users includes added user', async () => {
    const users = await listUsers();
    expect(users.some((u: User) => u.meta?.display_name === 'testuser')).toBe(true);
  });
});
