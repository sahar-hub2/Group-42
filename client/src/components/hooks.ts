/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import React, { useEffect, useState } from 'react';
import { Chat } from './types';
import { User } from '../platform/db';

interface ReceivedFileMeta {
  chunks: string[];
  name: string;
  size: number;
  received: number;
  total: number;
  blob?: Blob;
  assembled?: boolean;
  fileName?: string;
}

export function useOnlineUsers(user: User) {
  const [userDisplayNames, setUserDisplayNames] = useState<Record<string, string>>({});
  useEffect(() => {
    let polling = true;
    async function pollUsers() {
      while (polling) {
        try {
          const res = await fetch('http://localhost:3000/api/users/online');
          if (!res.ok) throw new Error('Failed to fetch online users');
          const data = await res.json();
          if (Array.isArray(data.users)) {
            const map: Record<string, string> = {};
            for (const u of data.users) {
              map[u.user_id] = u.display_name || u.user_id;
            }
            setUserDisplayNames(map);
          }
        } catch {
          console.error('Failed to fetch online users');
        }
        await new Promise((resolve) => setTimeout(resolve, 5000));
      }
    }
    pollUsers();
    return () => {
      polling = false;
    };
  }, [user]);
  return userDisplayNames;
}

export function usePollMessages(
  user: User,
  userDisplayNames: Record<string, string>,
  setChats: React.Dispatch<React.SetStateAction<Chat[]>>,
  chatsRef: React.MutableRefObject<Chat[]>,
  receivedFilesRef?: React.MutableRefObject<{ [file_id: string]: ReceivedFileMeta }>
) {
  const lastPublicMsgTsRef = React.useRef(0);
  const lastPublicFileTsRef = React.useRef(0);
  useEffect(() => {
    if (!user?.user_id) return;
    const interval = setInterval(async () => {
      try {
        // Poll direct messages
        const resp = await fetch('http://localhost:3000/api/poll_direct_messages', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ user_id: user.user_id }),
        });
        if (resp.ok) {
          const messages = await resp.json();
          if (Array.isArray(messages)) {
            for (const msg of messages) {
              if (msg.type === 'MSG_DIRECT' || msg.type === 'USER_DELIVER') {
                // For USER_DELIVER, use payload.sender; for MSG_DIRECT, use msg.from
                let senderId = '';
                if (msg.type === 'USER_DELIVER' && msg.payload && msg.payload.sender) {
                  senderId = msg.payload.sender;
                } else {
                  senderId =
                    typeof msg.from === 'string' ? msg.from : msg.from.Id || msg.from.id || '';
                }
                // Always resolve senderId to user_id if possible
                let resolvedUserId = senderId;
                let displayName = senderId;
                try {
                  const res = await fetch('http://localhost:3000/api/users/online');
                  if (res.ok) {
                    const data = await res.json();
                    if (Array.isArray(data.users)) {
                      const found = data.users.find(
                        (u: { user_id: string; display_name?: string }) =>
                          u.user_id === senderId || u.display_name === senderId
                      );
                      if (found) {
                        resolvedUserId = found.user_id;
                        displayName = found.display_name || found.user_id;
                      }
                    }
                  }
                } catch (e) {
                  console.error('Failed to fetch online users for display name resolution:', e);
                }
                // Decrypt message before adding to chat
                let text = '';
                const payload = msg.payload;
                if (payload && typeof payload === 'object') {
                  if (payload.ciphertext && user.privkey) {
                    try {
                      const privKey = user.privkey;
                      const b64 = payload.ciphertext;
                      const bin = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
                      const { importRsaPrivateKeyForDecrypt } = await import('../platform/crypto');
                      const key = await importRsaPrivateKeyForDecrypt(privKey);
                      const decrypted = await window.crypto.subtle.decrypt(
                        { name: 'RSA-OAEP' },
                        key,
                        bin
                      );
                      text = new TextDecoder().decode(decrypted);
                    } catch {
                      text = '[encrypted]';
                    }
                  } else if (payload.content) {
                    text = payload.content;
                  }
                }
                setChats((prev: Chat[]) => {
                  // Merge any chats with display name as id into user_id chat
                  const existing = prev.find((c) => c.id === resolvedUserId);
                  const dupe = prev.find((c) => c.id === senderId && senderId !== resolvedUserId);
                  let messages = existing ? [...existing.messages] : [];
                  if (dupe) {
                    messages = [...messages, ...dupe.messages];
                  }
                  // Add the new message (decrypted)
                  messages = [
                    ...messages,
                    {
                      id: Date.now() + Math.random(),
                      message: text,
                      senderId: resolvedUserId,
                      senderDisplayName: displayName,
                      direction: 'incoming',
                      timestamp: new Date().toLocaleTimeString([], {
                        hour: '2-digit',
                        minute: '2-digit',
                      }),
                    },
                  ];
                  // Remove duplicate chat if present
                  let filtered = prev.filter(
                    (c) => c.id !== senderId || senderId === resolvedUserId
                  );
                  // If chat doesn't exist, create it
                  if (!existing) {
                    filtered = [
                      ...filtered,
                      {
                        id: resolvedUserId,
                        name: displayName,
                        displayName,
                        messages,
                      },
                    ];
                  } else {
                    filtered = filtered.map((c) =>
                      c.id === resolvedUserId
                        ? { ...c, displayName, name: displayName, messages }
                        : c
                    );
                  }
                  return filtered;
                });
              } else if (
                msg.type === 'FILE_START' ||
                msg.type === 'FILE_CHUNK' ||
                msg.type === 'FILE_END'
              ) {
                // Only add a chat message for FILE_END (file ready to download)
                // Store chunk data in receivedFilesRef for assembly
                const fileId = msg.payload && msg.payload.file_id ? msg.payload.file_id : undefined;
                if (!fileId || !receivedFilesRef) return;
                if (msg.type === 'FILE_START') {
                  receivedFilesRef.current[fileId] = {
                    chunks: [],
                    name: msg.payload.name || 'attachment.bin',
                    size: msg.payload.size || 0,
                    received: 0,
                    total: 0,
                    blob: undefined,
                    assembled: false,
                    fileName: msg.payload.name || 'attachment.bin',
                  };
                } else if (msg.type === 'FILE_CHUNK') {
                  const fileMeta = receivedFilesRef.current[fileId];
                  if (fileMeta) {
                    fileMeta.chunks[msg.payload.index] = msg.payload.ciphertext;
                    fileMeta.received++;
                  }
                } else if (msg.type === 'FILE_END') {
                  const fileMeta = receivedFilesRef.current[fileId];
                  if (fileMeta) {
                    fileMeta.total = fileMeta.chunks.length;
                    // Only assemble if all chunks are present
                    if (
                      fileMeta.received === fileMeta.total &&
                      fileMeta.chunks.length === fileMeta.total &&
                      fileMeta.chunks.every((c: string) => typeof c === 'string') &&
                      !fileMeta.blob
                    ) {
                      const sortedChunks = fileMeta.chunks.map((b64) =>
                        Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))
                      );
                      const blob = new Blob(sortedChunks, { type: 'application/octet-stream' });
                      fileMeta.blob = blob;
                      fileMeta.assembled = true;
                    }
                    // Add a single chat message for the completed file
                    setChats((prev: Chat[]) => {
                      const existing = prev.find((c) => c.id === msg.from);
                      let messages = existing ? [...existing.messages] : [];
                      messages = [
                        ...messages,
                        {
                          id: Date.now() + Math.random(),
                          message: `📎 Received file (ID: ${fileId})`,
                          senderId: msg.from,
                          senderDisplayName: userDisplayNames[msg.from] || msg.from,
                          direction: 'incoming',
                          timestamp: new Date().toLocaleTimeString([], {
                            hour: '2-digit',
                            minute: '2-digit',
                          }),
                        },
                      ];
                      if (!existing) {
                        return [
                          ...prev,
                          {
                            id: msg.from,
                            name: userDisplayNames[msg.from] || msg.from,
                            displayName: userDisplayNames[msg.from] || msg.from,
                            messages,
                          },
                        ];
                      } else {
                        return prev.map((c) => (c.id === msg.from ? { ...c, messages } : c));
                      }
                    });
                  }
                }
              }
            }
          }
        }
        // Poll public channel messages independently
        // Track last seen public message timestamp using a ref
        const publicResp = await fetch(
          `http://localhost:3000/api/public_channel/messages?since=${lastPublicMsgTsRef.current}&exclude_from=${encodeURIComponent(user.user_id)}`,
          {
            method: 'GET',
          }
        );
        if (publicResp.ok) {
          const publicMessages = await publicResp.json();
          if (Array.isArray(publicMessages) && publicMessages.length > 0) {
            let maxTs = lastPublicMsgTsRef.current;
            for (const msg of publicMessages) {
              if (
                msg.sent_at &&
                typeof msg.sent_at === 'object' &&
                typeof msg.sent_at[0] === 'number'
              ) {
                maxTs = Math.max(maxTs, msg.sent_at[0]);
              }
              setChats((prev: Chat[]) => {
                const publicId = 'public';
                const publicName = 'Public Channel';
                const existing = prev.find((c) => c.id === publicId);
                let messages = existing ? [...existing.messages] : [];
                messages = [
                  ...messages,
                  {
                    id: Date.now() + Math.random(),
                    message: msg.content,
                    senderId: msg.from,
                    senderDisplayName: userDisplayNames[msg.from] || msg.from,
                    direction: 'incoming',
                    timestamp: new Date().toLocaleTimeString([], {
                      hour: '2-digit',
                      minute: '2-digit',
                    }),
                  },
                ];
                if (!existing) {
                  return [
                    ...prev,
                    {
                      id: publicId,
                      name: publicName,
                      displayName: publicName,
                      messages,
                    },
                  ];
                } else {
                  return prev.map((c) => (c.id === publicId ? { ...c, messages } : c));
                }
              });
            }
            lastPublicMsgTsRef.current = maxTs;
          }
        }

        // Poll public channel file events
        // lastPublicFileTsRef is now a useRef
        const fileResp = await fetch(
          `http://localhost:3000/api/public_channel/file_events?since=${lastPublicFileTsRef.current}`
        );
        if (fileResp.ok) {
          const fileEvents = await fileResp.json();
          if (Array.isArray(fileEvents) && fileEvents.length > 0) {
            let maxFileTs = lastPublicFileTsRef.current;
            for (const msg of fileEvents) {
              if (Array.isArray(msg.ts) && typeof msg.ts[0] === 'number') {
                maxFileTs = Math.max(maxFileTs, msg.ts[0]);
              }
              if (
                msg.type === 'FILE_START' ||
                msg.type === 'FILE_CHUNK' ||
                msg.type === 'FILE_END'
              ) {
                // Use the same logic as DM file handling, but always use 'public' as chat id
                const fileId = msg.payload && msg.payload.file_id ? msg.payload.file_id : undefined;
                if (!fileId || !receivedFilesRef) continue;
                if (msg.type === 'FILE_START') {
                  receivedFilesRef.current[fileId] = {
                    chunks: [],
                    name: msg.payload.name || 'attachment.bin',
                    size: msg.payload.size || 0,
                    received: 0,
                    total: 0,
                    blob: undefined,
                    assembled: false,
                    fileName: msg.payload.name || 'attachment.bin',
                  };
                } else if (msg.type === 'FILE_CHUNK') {
                  const fileMeta = receivedFilesRef.current[fileId];
                  if (fileMeta) {
                    fileMeta.chunks[msg.payload.index] = msg.payload.ciphertext;
                    fileMeta.received++;
                  }
                } else if (msg.type === 'FILE_END') {
                  const fileMeta = receivedFilesRef.current[fileId];
                  if (fileMeta) {
                    fileMeta.total = fileMeta.chunks.length;
                    if (
                      fileMeta.received === fileMeta.total &&
                      fileMeta.chunks.length === fileMeta.total &&
                      fileMeta.chunks.every((c: string) => typeof c === 'string') &&
                      !fileMeta.blob
                    ) {
                      const sortedChunks = fileMeta.chunks.map((b64) =>
                        Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))
                      );
                      const blob = new Blob(sortedChunks, { type: 'application/octet-stream' });
                      fileMeta.blob = blob;
                      fileMeta.assembled = true;
                    }
                    setChats((prev: Chat[]) => {
                      const publicId = 'public';
                      const publicName = 'Public Channel';
                      const existing = prev.find((c) => c.id === publicId);
                      let messages = existing ? [...existing.messages] : [];
                      messages = [
                        ...messages,
                        {
                          id: Date.now() + Math.random(),
                          message: `📎 Received file (ID: ${fileId})`,
                          senderId: publicId,
                          senderDisplayName: userDisplayNames[msg.from] || msg.from,
                          direction: 'incoming',
                          timestamp: new Date().toLocaleTimeString([], {
                            hour: '2-digit',
                            minute: '2-digit',
                          }),
                        },
                      ];
                      if (!existing) {
                        return [
                          ...prev,
                          {
                            id: publicId,
                            name: publicName,
                            displayName: publicName,
                            messages,
                          },
                        ];
                      } else {
                        return prev.map((c) => (c.id === publicId ? { ...c, messages } : c));
                      }
                    });
                  }
                }
              }
            }
            lastPublicFileTsRef.current = maxFileTs;
          }
        }
      } catch (e) {
        console.error('Failed to poll messages:', e);
      }
    }, 2000);
    return () => clearInterval(interval);
  }, [user, userDisplayNames, setChats, chatsRef]);
}

export function useHeartbeat(user: User) {
  useEffect(() => {
    if (!user?.user_id) return;
    const interval = setInterval(() => {
      fetch('http://localhost:3000/api/heartbeat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ user_id: user.user_id }),
      }).catch(() => {});
    }, 15000);
    return () => clearInterval(interval);
  }, [user]);
}

export function useUserHello(user: User) {
  useEffect(() => {
    const pemToB64Url = (pem: string) => {
      // Remove header/footer and newlines, then convert to b64url
      const b64 = (pem || '').replace(/-----.*?-----/g, '').replace(/\s+/g, '');
      // Convert base64 to base64url
      return b64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
    };
    const sendUserHello = async () => {
      try {
        if (
          !user.pubkey ||
          typeof user.pubkey !== 'string' ||
          !user.pubkey.startsWith('-----BEGIN PUBLIC KEY-----')
        ) {
          console.error('[USER_HELLO] Missing or invalid PEM public key:', user.pubkey);
          return;
        }
        const pubkeyB64Url = pemToB64Url(user.pubkey);
        const payload = {
          user_id: user.user_id,
          client: 'web-v1',
          pubkey: pubkeyB64Url,
          enc_pubkey: pubkeyB64Url, // duplicate for now
          meta: {
            display_name: String(user.meta?.display_name ?? user.user_id),
          },
        };
        await fetch('http://localhost:3000/api/user_hello', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(payload),
        });
      } catch (e) {
        console.error('Failed to send USER_HELLO', e);
      }
    };
    sendUserHello();
  }, [user]);
}
