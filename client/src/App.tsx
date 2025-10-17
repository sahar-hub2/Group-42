/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import './App.css';
import { useState } from 'react';

import '@chatscope/chat-ui-kit-styles/dist/default/styles.min.css';

import LoginScreen from './components/LogIn';
import ChatScreen from './components/ChatScreen';
import RegisterScreen from './components/Register';
import { User } from './platform/db';

function App() {
  const [currentUser, setCurrentUser] = useState<User | null>(null);
  const [showRegister, setShowRegister] = useState(false);

  if (currentUser) {
    return <ChatScreen onLogout={() => setCurrentUser(null)} user={currentUser} />;
  }

  return showRegister ? (
    <RegisterScreen
      onRegisterComplete={() => setShowRegister(false)} // go back to login after register
    />
  ) : (
    <LoginScreen
      onLogin={(user) => setCurrentUser(user)}
      onRegister={() => setShowRegister(true)} // switch to register UI
    />
  );
}

export default App;
