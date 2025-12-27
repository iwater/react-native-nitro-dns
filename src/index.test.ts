
import { setServers } from './src/index';

// Mock NitroModules
const mockNitroDns = {
    setServers: jest.fn(),
};

jest.mock('react-native-nitro-modules', () => ({
    NitroModules: {
        createHybridObject: () => mockNitroDns
    }
}));

describe('NitroDns JS API', () => {
    it('should correctly stringify servers in setServers', () => {
        const servers = ['tcp://8.8.8.8', 'udp://1.1.1.1'];
        setServers(servers);
        expect(mockNitroDns.setServers).toHaveBeenCalledWith(JSON.stringify(servers));
    });

    it('should handle IPv6 with protocol', () => {
        const servers = ['tcp://[::1]'];
        setServers(servers);
        expect(mockNitroDns.setServers).toHaveBeenCalledWith(JSON.stringify(servers));
    });
});
