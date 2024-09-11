import React, { useEffect } from 'react';
import { useWebSocket } from './WebSocketContext'; // Adjust path as necessary

const Chat: React.FC = () => {
    const socket = useWebSocket();

    useEffect(() => {
        if (socket) {
            socket.onmessage = (event) => {
                console.log('Received:', event.data);
            };
        }
    }, [socket]);

    const sendMessage = (message: string) => {
        if (socket) {
            socket.send(message);
        }
    };

    return (
        <div>
            <button onClick={() => sendMessage('Hello, server!')}>Send Message</button>
        </div>
    );
};

export default Chat;
