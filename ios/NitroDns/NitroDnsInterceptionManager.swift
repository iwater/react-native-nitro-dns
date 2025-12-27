import Foundation

@objc(NitroDnsInterceptionManager)
public class NitroDnsInterceptionManager: NSObject {
    @objc
    public static let shared = NitroDnsInterceptionManager()

    private var isEnabled = false

    @objc
    public func setEnabled(_ enabled: Bool) {
        if enabled == isEnabled { return }
        isEnabled = enabled
        
        if enabled {
            URLProtocol.registerClass(NitroDnsURLProtocol.self)
        } else {
            URLProtocol.unregisterClass(NitroDnsURLProtocol.self)
        }
    }
    
    @objc 
    public func isInterceptionEnabled() -> Bool {
        return isEnabled
    }
}

@_cdecl("nitro_dns_set_interception_enabled_ios")
public func nitro_dns_set_interception_enabled_ios(_ enabled: Bool) {
    NitroDnsInterceptionManager.shared.setEnabled(enabled)
}
