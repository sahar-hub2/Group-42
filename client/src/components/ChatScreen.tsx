/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import { useState, useEffect, useRef } from 'react';
import { OnlineUsers } from './OnlineUsers';
import {
  MainContainer,
  ConversationList,
  Conversation,
  Sidebar,
} from '@chatscope/chat-ui-kit-react';
import { Chat } from './types';
import { useOnlineUsers, usePollMessages, useHeartbeat, useUserHello } from './hooks';
import { decryptPrivateKey } from '../platform/privkey_decrypt';
import { User } from '../platform/db';
import { sendFileDM } from '../platform/sendFileDM';
import { handleSend } from '../platform/handleSend';
import { publicChannelSend } from '../platform/publicChannelSend';
import FileMessageList from './FileMessageList';
import { setChatsWithFiles } from '../platform/setChatsWithFiles';
import { sendFilePublicChannel } from '../platform/sendFilePublicChannel';

function ChatScreen({ onLogout, user }: { onLogout: () => void; user: User }) {
  const sentFilesRef = useRef<{ [file_id: string]: Blob }>({});
  const receivedFilesRef = useRef<{
    [file_id: string]: {
      chunks: string[];
      name: string;
      size: number;
      received: number;
      total: number;
      blob?: Blob;
      assembled?: boolean;
      fileName?: string;
    };
  }>({});
  // Add a default public channel chat
  const PUBLIC_CHANNEL_ID = 'public';
  const PUBLIC_CHANNEL_NAME = 'Public Channel';
  const [chats, setChats] = useState<Chat[]>([
    {
      id: PUBLIC_CHANNEL_ID,
      name: PUBLIC_CHANNEL_NAME,
      displayName: PUBLIC_CHANNEL_NAME,
      messages: [],
    },
  ]);
  const chatsRef = useRef<Chat[]>([]);
  const userDisplayNames = useOnlineUsers(user);
  const [, setFileAssemblyTick] = useState(0);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Shows only the selected chat
  const [activeChatId, setActiveChatId] = useState<string>('1');
  const [showNewChatModal, setShowNewChatModal] = useState<boolean>(false);
  const [showAttachModal, setShowAttachModal] = useState<boolean>(false);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);

  const activeChat = chats.find((c: Chat) => c.id === activeChatId);

  const getCurrentTime = () =>
    new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

  usePollMessages(
    user,
    userDisplayNames,
    (updater) => setChatsWithFiles(updater, setChats, receivedFilesRef, setFileAssemblyTick),
    chatsRef,
    receivedFilesRef
  );
  useHeartbeat(user);
  useUserHello(user);

  // Send message handler (direct or public channel)
  const handleSendMessage = async (text: string) => {
    if (activeChatId === PUBLIC_CHANNEL_ID) {
      // Send to public channel
      await publicChannelSend(
        text,
        chats,
        setChats,
        user,
        setErrorMsg,
        getCurrentTime,
        PUBLIC_CHANNEL_ID
      );
    } else {
      await handleSend(text, chats, setChats, user, setErrorMsg, activeChatId, getCurrentTime);
    }
  };

  const handleAttach = () => {
    setShowAttachModal(true);
  };

  const handleConfirmAttachment = async () => {
    if (selectedFile) {
      // Find the recipient user ID for the active chat
      let chat = chats.find((c: Chat) => c.id === activeChatId);
      if (!chat) {
        setErrorMsg('No active chat selected.');
        setShowAttachModal(false);
        return;
      }
      // If public channel, use public channel file upload
      if (activeChatId === PUBLIC_CHANNEL_ID) {
        try {
          await sendFilePublicChannel(selectedFile, user, sentFilesRef, PUBLIC_CHANNEL_ID);
          setChats((prev: Chat[]) =>
            prev.map((chat: Chat) =>
              chat.id === PUBLIC_CHANNEL_ID
                ? {
                    ...chat,
                    messages: [
                      ...chat.messages,
                      {
                        id: Date.now(),
                        message: `📎 Sent file: ${selectedFile.name}`,
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
          setErrorMsg(null);
        } catch (e) {
          setErrorMsg('Failed to send file: ' + (e instanceof Error ? e.message : String(e)));
        }
        setSelectedFile(null);
        setShowAttachModal(false);
        return;
      }
      let recipientUserId = chat.id;
      // If chat.id is not a valid user_id, resolve it
      if (!/^[a-zA-Z0-9_-]{8,}$/.test(recipientUserId)) {
        try {
          const res = await fetch('http://localhost:3000/api/users/online');
          if (res.ok) {
            const data = await res.json();
            if (Array.isArray(data.users)) {
              const found = data.users.find(
                (u: { user_id: string; display_name?: string }) =>
                  u.display_name === recipientUserId
              );
              if (found) {
                recipientUserId = found.user_id;
                setChats((prev) =>
                  prev.map((c) => (c.id === chat!.id ? { ...c, id: found.user_id } : c))
                );
                chat = { ...chat!, id: found.user_id };
              }
            }
          }
        } catch {
          setErrorMsg('Failed to resolve display name to user_id for file transfer.');
          setShowAttachModal(false);
          return;
        }
      }
      if (!recipientUserId || recipientUserId === user.user_id) {
        setErrorMsg('Cannot send a file to yourself.');
        setShowAttachModal(false);
        return;
      }
      try {
        await sendFileDM(selectedFile, recipientUserId, user, sentFilesRef);
        setChats((prev: Chat[]) =>
          prev.map((chat: Chat) =>
            chat.id === activeChatId
              ? {
                  ...chat,
                  messages: [
                    ...chat.messages,
                    {
                      id: Date.now(),
                      message: `📎 Sent file: ${selectedFile.name}`,
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
        setErrorMsg(null);
      } catch (e) {
        setErrorMsg('Failed to send file: ' + (e instanceof Error ? e.message : String(e)));
      }
      setSelectedFile(null);
    }
    setShowAttachModal(false);
  };

  const handleCancelAttachment = () => {
    setSelectedFile(null);
    setShowAttachModal(false);
  };

  const handleAddChat = async (userId: string, displayName: string) => {
    // Prevent duplicate chats with the same user
    const existingChat = chats.find((c: Chat) => c.id === userId);
    if (existingChat) {
      setActiveChatId(existingChat.id);
    } else {
      // Fetch recipient's public key from server
      let pubkey = '';
      try {
        const res = await fetch(
          `http://localhost:3000/api/users/pubkey/${encodeURIComponent(userId)}`
        );
        if (res.ok) {
          const data = await res.json();
          pubkey = data.pubkey || '';
        }
      } catch (e) {
        console.error('Failed to fetch recipient public key:', e);
      }
      if (!pubkey) {
        setErrorMsg('Could not fetch recipient public key. Cannot start chat.');
        return;
      }
      const newChat: Chat = {
        id: userId,
        name: displayName,
        displayName,
        messages: [],
        pubkey,
      };
      setChats((prev: Chat[]) => [...prev, newChat]);
      setActiveChatId(userId);
    }
    setShowNewChatModal(false);
  };

  // Load private key into memory if missing
  useEffect(() => {
    if (!user.privkey && user.privkey_store && user.password) {
      (async () => {
        try {
          const priv = await decryptPrivateKey(user.privkey_store, user.password);
          user.privkey = priv;
        } catch (e) {
          console.error('Failed to decrypt private key:', e);
        }
      })();
    }
  }, [user]);

  // Update chats' displayName and name fields when userDisplayNames changes
  useEffect(() => {
    setChats((prevChats) =>
      prevChats.map((chat) => {
        const newDisplayName = userDisplayNames[chat.id] || chat.id;
        if (chat.displayName !== newDisplayName || chat.name !== newDisplayName) {
          return { ...chat, displayName: newDisplayName, name: newDisplayName };
        }
        return chat;
      })
    );
  }, [userDisplayNames]);

  useEffect(() => {
    chatsRef.current = chats;
  }, [chats]);

  return (
    <div className="chat-container">
      {errorMsg && (
        <div className="chat-error" style={{ color: 'red', margin: '0.5rem 1rem' }}>
          {errorMsg}
        </div>
      )}
      <div
        style={{ padding: '0.5rem 1rem', background: '#f3f4f6', borderBottom: '1px solid #e5e7eb' }}
      >
        <span>
          Logged in as: <strong>{String(user.meta?.display_name || user.user_id || '')}</strong>
        </span>
      </div>
      <MainContainer responsive>
        <div className="sidebar-wrapper">
          {/* sidebar with all chats the user has */}
          <Sidebar position="left" scrollable>
            <div className="sidebar-header">
              <strong>Chats</strong>
              <button className="newchat-button" onClick={() => setShowNewChatModal(true)}>
                + New Chat
              </button>
              <button className="logout-button" onClick={onLogout}>
                Logout
              </button>
            </div>
            <ConversationList>
              {chats.map((chat: Chat) => (
                <Conversation
                  key={chat.id}
                  name={chat.displayName}
                  info={chat.messages[chat.messages.length - 1]?.message || 'No messages'}
                  active={chat.id === activeChatId}
                  onClick={() => setActiveChatId(chat.id)}
                />
              ))}
            </ConversationList>
          </Sidebar>
        </div>

        {/* chat area */}
        <FileMessageList
          activeChat={activeChat}
          sentFilesRef={sentFilesRef}
          receivedFilesRef={receivedFilesRef}
          chats={chats}
          setChats={setChats}
          user={user}
          setErrorMsg={setErrorMsg}
          activeChatId={activeChatId}
          getCurrentTime={getCurrentTime}
          handleSend={handleSendMessage}
          handleAttach={handleAttach}
        />
      </MainContainer>

      {/* New Chat Modal */}
      {showNewChatModal && (
        <div className="modal-overlay">
          <div className="modal">
            <h3>Select a user to chat with</h3>
            <OnlineUsers user={user} onSelectUser={handleAddChat} />
            <div className="modal-buttons">
              <button onClick={() => setShowNewChatModal(false)}>Cancel</button>
            </div>
          </div>
        </div>
      )}
      {/* Attachment Modal */}
      {showAttachModal && (
        <div className="modal-overlay">
          <div className="modal">
            <h3>Upload Attachment</h3>
            <input
              type="file"
              onChange={(e) => setSelectedFile(e.target.files ? e.target.files[0] : null)}
            />
            {selectedFile && (
              <p>
                Selected file: <strong>{selectedFile.name}</strong>
              </p>
            )}
            <div className="modal-buttons">
              <button onClick={handleConfirmAttachment} disabled={!selectedFile}>
                Send
              </button>
              <button onClick={handleCancelAttachment}>Cancel</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default ChatScreen;
