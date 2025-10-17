/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

export type ChatMessage = {
  id: number;
  message: string;
  senderId: string;
  senderDisplayName: string;
  direction: 'incoming' | 'outgoing';
  timestamp: string;
};

export type Chat = {
  id: string;
  name: string; // userId or displayName
  displayName: string;
  messages: ChatMessage[];
  pubkey?: string; // recipient's public key (PEM)
};

export type User = {
  user_id: string;
  pubkey?: string; // PEM format
  privkey?: string; // PEM format, decrypted private key (only in memory)
  privkey_store?: string; // Encrypted private key storage (AES-GCM)
  password?: string; // PBKDF2 hashed password
  salt?: string; // Salt used for PBKDF2
  meta?: {
    display_name?: string;
    [key: string]: unknown;
  };
  version?: number;
  [key: string]: unknown;
};
