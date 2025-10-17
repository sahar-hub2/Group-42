/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import { encryptWithPublicKey, signContent } from '../platform/crypto';
import { Chat, User } from '../components/types';
import { Dispatch, SetStateAction } from 'react';

// Send direct message to server
export const handleSend = async (
  text: string,
  chats: Chat[],
  setChats: Dispatch<SetStateAction<Chat[]>>,
  user: User,
  setErrorMsg: (msg: string | null) => void,
  activeChatId: string,
  getCurrentTime: () => string
) => {
  let chat = chats.find((c: Chat) => c.id === activeChatId);
  if (!chat) return;
  // Determine recipient user ID
  let recipientUserId = chat.id;
  // If chat.id is not a valid user_id (e.g., it's a display name), resolve it
  if (!/^[a-zA-Z0-9_-]{8,}$/.test(recipientUserId)) {
    // Query /api/users/online to resolve display name to user_id
    try {
      const res = await fetch('http://localhost:3000/api/users/online');
      if (res.ok) {
        const data = await res.json();
        if (Array.isArray(data.users)) {
          const found = data.users.find(
            (u: { user_id: string; display_name?: string }) => u.display_name === recipientUserId
          );
          if (found) {
            recipientUserId = found.user_id;
            // Also update chat.id to be the user_id for future
            setChats((prev) =>
              prev.map((c) => (c.id === chat!.id ? { ...c, id: found.user_id } : c))
            );
            chat = { ...chat!, id: found.user_id };
          }
        }
      }
    } catch (e) {
      console.error('Failed to resolve display name to user_id:', e);
    }
  }
  if (!recipientUserId || recipientUserId === user.user_id) {
    setErrorMsg('Cannot send a direct message to yourself.');
    return;
  }
  // Lookup recipient's public key
  let recipientPubKey = chat.pubkey;
  if (!recipientPubKey) {
    // Try to fetch the pubkey from the server
    try {
      const res = await fetch(
        `http://localhost:3000/api/users/pubkey/${encodeURIComponent(recipientUserId)}`
      );
      if (res.ok) {
        const data = await res.json();
        recipientPubKey = data.pubkey || '';
        if (recipientPubKey) {
          // Update chat with pubkey
          setChats((prev) =>
            prev.map((c) => (c.id === recipientUserId ? { ...c, pubkey: recipientPubKey } : c))
          );
        }
      }
    } catch (e) {
      console.error('Failed to fetch recipient public key:', e);
    }
    if (!recipientPubKey) {
      setErrorMsg('Recipient public key not found. Cannot send encrypted message.');
      return;
    }
  }
  // If the key is base64url, convert it back to PEM
  if (!recipientPubKey.startsWith('-----BEGIN PUBLIC KEY-----')) {
    // Convert base64url to base64
    let b64 = recipientPubKey.replace(/-/g, '+').replace(/_/g, '/');
    while (b64.length % 4) b64 += '=';
    // Format as PEM
    recipientPubKey =
      '-----BEGIN PUBLIC KEY-----\n' +
      b64.match(/.{1,64}/g)?.join('\n') +
      '\n-----END PUBLIC KEY-----';
  }
  // Encrypt message with recipient's public key
  let ciphertext = '';
  try {
    ciphertext = await encryptWithPublicKey(recipientPubKey, text);
  } catch (e) {
    setErrorMsg('Encryption failed: ' + e);
    return;
  }
  // Sign the plaintext content with sender's private key
  if (!user.privkey) {
    setErrorMsg('Your private key is missing. Cannot sign message.');
    return;
  }
  let contentSig = '';
  try {
    contentSig = await signContent(user.privkey, text);
  } catch (e) {
    setErrorMsg('Signing failed: ' + e);
    return;
  }
  // Compose payload
  const now = Date.now();
  const msgPayload = {
    type: 'MSG_DIRECT',
    from: user.user_id,
    to: recipientUserId,
    ts: [Math.floor(now / 1000), (now % 1000) * 1_000_000],
    payload: {
      ciphertext,
      sender_pub: user.pubkey || '',
      content_sig: contentSig,
    },
    sig: '',
  };
  try {
    await fetch('http://localhost:3000/api/direct_message', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(msgPayload),
    });
  } catch (err) {
    setErrorMsg(
      'Failed to send direct message: ' + (err instanceof Error ? err.message : String(err))
    );
    return;
  }
  setErrorMsg(null);
  // Add outgoing message
  setChats((prev: Chat[]) =>
    prev.map((chat: Chat) =>
      chat.id === activeChatId
        ? {
            ...chat,
            messages: [
              ...chat.messages,
              {
                id: Date.now(),
                message: text,
                senderId: user.user_id,
                senderDisplayName: String(user.meta?.display_name ?? 'Me'),
                direction: 'outgoing',
                timestamp: getCurrentTime(),
              },
            ],
          }
        : chat
    )
  );
};
