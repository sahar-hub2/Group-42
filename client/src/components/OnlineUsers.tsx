/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import { useEffect, useState } from 'react';
import { User } from '../platform/db';

interface OnlineUsersProps {
  user: User;
  onSelectUser?: (userId: string, displayName: string) => void;
}

interface ApiUser {
  user_id: string;
  display_name?: string;
}

export function OnlineUsers({ user, onSelectUser }: OnlineUsersProps) {
  const [onlineUsers, setOnlineUsers] = useState<{ user_id: string; displayName: string }[]>([]);

  useEffect(() => {
    let polling = true;
    async function pollUsers() {
      while (polling) {
        try {
          const res = await fetch('http://localhost:3000/api/users/online');
          if (!res.ok) throw new Error('Failed to fetch online users');
          const data = await res.json();
          if (Array.isArray(data.users)) {
            setOnlineUsers(
              data.users.map((u: ApiUser) => ({
                user_id: u.user_id,
                displayName: u.display_name || u.user_id,
              }))
            );
          } else {
            setOnlineUsers([]);
          }
        } catch {
          setOnlineUsers([]);
        }
        await new Promise((resolve) => setTimeout(resolve, 5000));
      }
    }
    pollUsers();
    return () => {
      polling = false;
    };
  }, [user]);

  const filteredUsers = onlineUsers.filter((u) => u.user_id !== user.user_id);
  return (
    <ul className="user-list">
      {filteredUsers.length === 0 ? (
        <li style={{ color: '#888', textAlign: 'center' }}>No online users</li>
      ) : (
        filteredUsers.map((u) => (
          <li key={u.user_id}>
            {onSelectUser ? (
              <button onClick={() => onSelectUser(u.user_id, u.displayName)}>
                {u.displayName}
              </button>
            ) : (
              u.displayName
            )}
          </li>
        ))
      )}
    </ul>
  );
}
