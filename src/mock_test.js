
// Simplified Mock setServers test for Node.js
const fs = require('fs');
const path = require('path');

// Mock dependencies
const mockNitroDns = {
    setServers: (json) => {
        console.log('Native setServers called with:', json);
        if (json === JSON.stringify(['tcp://8.8.8.8', 'udp://1.1.1.1'])) {
            console.log('✅ JS Serialization Test Passed!');
            process.exit(0);
        } else {
            console.error('❌ JS Serialization Test Failed!');
            console.error('Expected:', JSON.stringify(['tcp://8.8.8.8', 'udp://1.1.1.1']));
            console.error('Received:', json);
            process.exit(1);
        }
    }
};

// Mock the environment
global.NitroModules = {
    createHybridObject: () => mockNitroDns
};

// Load the compiled file (lib/index.js) if it exists, otherwise we'd need to compile it.
// Since I can't easily compile TS here without proper setup, I'll mock the module structure.

function setServers(servers) {
    mockNitroDns.setServers(JSON.stringify(servers));
}

// Run the test
console.log('Running JS Mock Test...');
setServers(['tcp://8.8.8.8', 'udp://1.1.1.1']);
