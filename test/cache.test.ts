
import { Resolver, clearCache } from '../src/index';

// Mock NitroModules
const mockNitroResolver = {
    cancel: jest.fn(),
    getServers: jest.fn().mockReturnValue('[]'),
    setServers: jest.fn(),
    setLocalAddress: jest.fn(),
    clearCache: jest.fn(),
};

const mockNitroDns = {
    createResolver: jest.fn().mockReturnValue(mockNitroResolver),
    clearCache: jest.fn(),
};

jest.mock('react-native-nitro-modules', () => ({
    NitroModules: {
        createHybridObject: () => mockNitroDns
    }
}));

describe('NitroDns Cache API', () => {
    it('should pass cacheSize to createResolver', () => {
        const resolver = new Resolver({ cacheSize: 1000 });
        expect(mockNitroDns.createResolver).toHaveBeenCalledWith(JSON.stringify({ cacheSize: 1000 }));
    });

    it('should call native clearCache on Resolver', () => {
        const resolver = new Resolver();
        resolver.clearCache();
        expect(mockNitroResolver.clearCache).toHaveBeenCalled();
    });

    it('should call native global clearCache', () => {
        clearCache();
        expect(mockNitroDns.clearCache).toHaveBeenCalled();
    });
});
