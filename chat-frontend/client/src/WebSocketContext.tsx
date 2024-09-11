import React, { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import { w3cwebsocket as W3CWebSocket } from 'websocket';

const WebSocketContext = createContext<W3CWebSocket | null>(null);

export const useWebSocket = () => {
    return useContext(WebSocketContext);
};

interface WebSocketProviderProps {
    children: ReactNode; // This allows any valid React child type
}

export const WebSocketProvider: React.FC<WebSocketProviderProps> = ({ children }) => {
    const [client, setClient] = useState<W3CWebSocket | null>(null);

    useEffect(() => {
        const socket = new W3CWebSocket('ws://127.0.0.1:8080');

        socket.onopen = () => {
            console.log('WebSocket Client Connected');
        };

        socket.onclose = () => {
            console.log('WebSocket Client Disconnected');
        };

        setClient(socket);

        return () => {
            socket.close();
        };
    }, []);

    return (
        <WebSocketContext.Provider value={client}>
            {children}
        </WebSocketContext.Provider>
    );
};
