/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

// Cross-platform DB logic for users table
// Tauri: uses SQLite via @tauri-apps/plugin-sql
// Web: uses backend REST API

import { isTauri } from './index';

// User type for DB
export type User = {
  user_id: string;
  pubkey: string;
  privkey_store: string;
  privkey?: string;
  password: string;
  salt: string;
  meta?: Record<string, unknown>;
  version: number;
};

// --- Tauri (desktop) implementation ---
async function tauriDb() {
  const Database = (await import('@tauri-apps/plugin-sql')).default;
  return Database.load('sqlite:secure_chat.db');
}

// --- Browser (IndexedDB) implementation ---
const DB_NAME = 'secure_chat_web';
const STORE_NAME = 'users';
const DB_VERSION = 1;

function idbPromisify<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function idbDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const open = indexedDB.open(DB_NAME, DB_VERSION);
    open.onupgradeneeded = () => {
      const db = open.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME, { keyPath: 'user_id' });
      }
    };
    open.onsuccess = () => resolve(open.result);
    open.onerror = () => reject(open.error);
  });
}

export async function addUser(user: User): Promise<void> {
  if (isTauri()) {
    const db = await tauriDb();
    await db.execute(
      `INSERT INTO users (user_id, pubkey, privkey_store, password, salt, meta, version) VALUES (?, ?, ?, ?, ?, ?, ?)`,
      [
        user.user_id,
        user.pubkey,
        user.privkey_store,
        user.password,
        user.salt,
        user.meta ? JSON.stringify(user.meta) : null,
        user.version,
      ]
    );
  } else if (typeof indexedDB !== 'undefined') {
    const db = await idbDb();
    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    await idbPromisify(store.put(user));
    await new Promise<void>((resolve, reject) => {
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error);
    });
  } else {
    throw new Error('addUser: No supported storage backend found.');
  }
}

export async function getUserById(user_id: string): Promise<User | null> {
  if (isTauri()) {
    const db = await tauriDb();
    const rows = await db.select<User[]>(`SELECT * FROM users WHERE user_id = ?`, [user_id]);
    if (rows.length === 0) return null;
    const user = rows[0];
    if (user.meta && typeof user.meta === 'string') user.meta = JSON.parse(user.meta);
    return user;
  } else if (typeof indexedDB !== 'undefined') {
    const db = await idbDb();
    const tx = db.transaction(STORE_NAME, 'readonly');
    const store = tx.objectStore(STORE_NAME);
    const user = await idbPromisify(store.get(user_id));
    await new Promise<void>((resolve, reject) => {
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error);
    });
    return user || null;
  } else {
    throw new Error('getUserById: No supported storage backend found.');
  }
}

export async function listUsers(): Promise<User[]> {
  if (isTauri()) {
    const db = await tauriDb();
    const rows = await db.select<User[]>(`SELECT * FROM users`);
    return rows.map((user) => {
      if (user.meta && typeof user.meta === 'string') user.meta = JSON.parse(user.meta);
      return user;
    });
  } else if (typeof indexedDB !== 'undefined') {
    const db = await idbDb();
    const tx = db.transaction(STORE_NAME, 'readonly');
    const store = tx.objectStore(STORE_NAME);
    const users: User[] = await idbPromisify(store.getAll());
    await new Promise<void>((resolve, reject) => {
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error);
    });
    return users;
  } else {
    throw new Error('listUsers: No supported storage backend found.');
  }
}
