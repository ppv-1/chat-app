import React from 'react';
import './App.css';
import Chat from './Chat';

const App: React.FC = () => {
    return (
        <div className="App">
            <header className="App-header">
                <h1>Chat Application</h1>
                <Chat />
            </header>
        </div>
    );
};

export default App;
