/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import { Chat, User } from '../components/types';
import { Dispatch, SetStateAction } from 'react';

// Send public channel message to server

export const publicChannelSend = async (
  text: string,
  chats: Chat[],
  setChats: Dispatch<SetStateAction<Chat[]>>,
  user: User,
  setErrorMsg: (msg: string | null) => void,
  getCurrentTime: () => string,
  channelId: string
) => {
  // Compose payload for HTTP handler: { channel_id, from, content }
  const msgPayload = {
    channel_id: channelId,
    from: user.user_id,
    content: text,
  };
  try {
    await fetch('http://localhost:3000/api/public_channel/message', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(msgPayload),
    });
    setErrorMsg(null);
  } catch (err) {
    setErrorMsg(
      'Failed to send public channel message: ' + (err instanceof Error ? err.message : String(err))
    );
    return;
  }
  // Add outgoing message to public channel chat
  setChats((prev: Chat[]) =>
    prev.map((chat: Chat) =>
      chat.id === channelId
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
