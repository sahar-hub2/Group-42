/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import React from 'react';
import { MessageList, Message, MessageInput, ChatContainer } from '@chatscope/chat-ui-kit-react';
import { Chat, User } from './types';

interface FileMessageListProps {
  activeChat: Chat | undefined;
  sentFilesRef: React.RefObject<{ [file_id: string]: Blob }>;
  receivedFilesRef: React.RefObject<{
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
  }>;
  chats: Chat[];
  setChats: React.Dispatch<React.SetStateAction<Chat[]>>;
  user: User;
  setErrorMsg: (msg: string | null) => void;
  activeChatId: string;
  getCurrentTime: () => string;
  handleSend: (
    text: string,
    chats: Chat[],
    setChats: React.Dispatch<React.SetStateAction<Chat[]>>,
    user: User,
    setErrorMsg: (msg: string | null) => void,
    activeChatId: string,
    getCurrentTime: () => string
  ) => Promise<void>;
  handleAttach: () => void;
}

const FileMessageList: React.FC<FileMessageListProps> = ({
  activeChat,
  sentFilesRef,
  receivedFilesRef,
  chats,
  setChats,
  user,
  setErrorMsg,
  activeChatId,
  getCurrentTime,
  handleSend,
  handleAttach,
}) => (
  <ChatContainer>
    <MessageList>
      {activeChat
        ? activeChat.messages.map(
            (m: {
              id: number;
              message: string;
              senderId: string;
              senderDisplayName: string;
              direction: 'incoming' | 'outgoing';
              timestamp: string;
            }) => {
              let fileId: string | undefined = undefined;
              let fileName: string | undefined = undefined;
              if (m.message.startsWith('📎')) {
                const match = m.message.match(/ID: ([a-f0-9-]+)/);
                if (match) fileId = match[1];
                const nameMatch = m.message.match(/file: ([^)]+)/);
                if (nameMatch) fileName = nameMatch[1];
                if (fileId && !fileName && receivedFilesRef.current[fileId]?.fileName) {
                  fileName = receivedFilesRef.current[fileId].fileName;
                }
              }
              return (
                <Message
                  key={m.id}
                  model={{
                    message: m.message,
                    sentTime: m.timestamp,
                    sender: m.senderDisplayName,
                    direction: m.direction,
                    position: 'single',
                  }}
                >
                  <Message.Header sender={m.senderDisplayName} sentTime={m.timestamp} />
                  {m.message.startsWith('📎') && fileId && (
                    <Message.CustomContent>
                      <span style={{ marginRight: 8, fontWeight: 500 }}>
                        {fileName || 'attachment.bin'}
                      </span>
                      {(() => {
                        const senderReady = fileId && sentFilesRef.current[fileId];
                        const recipientReady =
                          fileId &&
                          receivedFilesRef.current[fileId] &&
                          receivedFilesRef.current[fileId].assembled &&
                          receivedFilesRef.current[fileId].blob;
                        if (fileId && (senderReady || recipientReady)) {
                          return (
                            <button
                              style={{ marginLeft: 0 }}
                              onClick={async () => {
                                if (senderReady) {
                                  const url = URL.createObjectURL(sentFilesRef.current[fileId]);
                                  const a = document.createElement('a');
                                  a.href = url;
                                  a.download = fileName || 'attachment.bin';
                                  a.click();
                                  URL.revokeObjectURL(url);
                                } else if (recipientReady) {
                                  const url = URL.createObjectURL(
                                    receivedFilesRef.current[fileId].blob!
                                  );
                                  const a = document.createElement('a');
                                  a.href = url;
                                  a.download =
                                    receivedFilesRef.current[fileId]?.fileName || 'attachment.bin';
                                  a.click();
                                  URL.revokeObjectURL(url);
                                }
                              }}
                            >
                              Download
                            </button>
                          );
                        } else {
                          return (
                            <button
                              style={{ marginLeft: 0 }}
                              disabled
                              title="Waiting for file to be received..."
                            >
                              <span
                                className="spinner"
                                style={{
                                  marginRight: 4,
                                  display: 'inline-block',
                                  width: 12,
                                  height: 12,
                                  border: '2px solid #ccc',
                                  borderTop: '2px solid #888',
                                  borderRadius: '50%',
                                  animation: 'spin 1s linear infinite',
                                }}
                              />
                              Receiving...
                            </button>
                          );
                        }
                      })()}
                    </Message.CustomContent>
                  )}
                </Message>
              );
            }
          )
        : null}
    </MessageList>
    <MessageInput
      placeholder={activeChat ? 'Type a message...' : 'Select a chat to start messaging...'}
      onSend={async (text) => {
        if (!activeChat) return;
        await handleSend(text, chats, setChats, user, setErrorMsg, activeChatId, getCurrentTime);
      }}
      onAttachClick={handleAttach}
      disabled={!activeChat}
    />
  </ChatContainer>
);

export default FileMessageList;
