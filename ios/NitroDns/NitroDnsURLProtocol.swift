import Foundation
import DnsBridge

/// 基于 KIDDNS RxSNIURLProtocol 的实现方式
/// 使用 CFHTTPMessage 和 CFReadStreamCreateForHTTPRequest 处理 HTTPS SNI 场景
@objc(NitroDnsURLProtocol)
public class NitroDnsURLProtocol: URLProtocol, StreamDelegate {
    private static let key = "NitroDnsURLProtocolHandled"
    
    private var inputStream: InputStream?
    private var currentRunLoop: RunLoop?
    private var runLoopMode: RunLoop.Mode = .common
    
    private var originalHost: String?
    private var originalURL: URL?
    private var requestWithIP: URLRequest?
    private var requestWithDomain: URLRequest?
    private var currentRequest: URLRequest?
    private var headerComplete = false
    
    // MARK: - URLProtocol Override
    
    public override class func canInit(with request: URLRequest) -> Bool {
        // Only intercept if enabled via JS
        guard NitroDnsInterceptionManager.shared.isInterceptionEnabled() else { return false }
        
        guard let url = request.url, let scheme = url.scheme else { return false }
        if scheme != "http" && scheme != "https" { return false }
        
        // Skip if already handled to avoid recursion
        if URLProtocol.property(forKey: key, in: request) != nil {
            return false
        }
        
        // Skip IP addresses - only intercept hostnames
        if let host = url.host {
            let ipv4Pattern = "^\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}$"
            if host.range(of: ipv4Pattern, options: .regularExpression) != nil {
                return false
            }
        }
        
        NSLog("[NitroDns] canInit for: %@", url.absoluteString)
        return true
    }

    public override class func canonicalRequest(for request: URLRequest) -> URLRequest {
        return request
    }

    public override func startLoading() {
        guard let mutableRequest = (request as NSURLRequest).mutableCopy() as? NSMutableURLRequest,
              let url = mutableRequest.url,
              let host = url.host else {
            client?.urlProtocol(self, didFailWithError: NSError(domain: "NitroDns", code: -1, userInfo: [NSLocalizedDescriptionKey: "Invalid URL"]))
            return
        }
        
        // Mark as handled
        URLProtocol.setProperty(true, forKey: NitroDnsURLProtocol.key, in: mutableRequest)
        
        originalHost = host
        originalURL = url
        requestWithDomain = mutableRequest as URLRequest
        
        NSLog("[NitroDns] startLoading for: %@ (host: %@)", url.absoluteString, host)
        
        // Resolve DNS using Rust FFI
        if let resolvedIP = resolveDNS(host: host) {
            NSLog("[NitroDns] DNS resolved: %@ -> %@", host, resolvedIP)
            
            // Set Host header
            mutableRequest.setValue(host, forHTTPHeaderField: "Host")
            
            // Replace host with IP in URL
            var components = URLComponents(url: url, resolvingAgainstBaseURL: false)
            components?.host = resolvedIP
            if let newURL = components?.url {
                mutableRequest.url = newURL
                requestWithIP = mutableRequest as URLRequest
            }
        }
        
        startRequest()
    }
    
    private func startRequest() {
        if let req = requestWithIP {
            startRequest(req)
        } else if let req = requestWithDomain {
            startRequest(req)
        }
    }
    
    private func startRequest(_ request: URLRequest) {
        currentRequest = request
        
        guard let url = request.url else { return }
        
        // Build CFHTTPMessage
        let httpMethod = (request.httpMethod ?? "GET") as CFString
        let requestURL = url as CFURL
        
        let cfRequest = CFHTTPMessageCreateRequest(kCFAllocatorDefault, httpMethod, requestURL, kCFHTTPVersion1_1).takeRetainedValue()
        
        // Set body
        if let body = request.httpBody {
            CFHTTPMessageSetBody(cfRequest, body as CFData)
        }
        
        // Set headers
        if let headers = request.allHTTPHeaderFields {
            for (key, value) in headers {
                CFHTTPMessageSetHeaderFieldValue(cfRequest, key as CFString, value as CFString)
            }
        }
        
        // Handle cookies for original domain
        if let originalURL = originalURL {
            let cookies = HTTPCookieStorage.shared.cookies(for: originalURL) ?? []
            let cookieHeaders = HTTPCookie.requestHeaderFields(with: cookies)
            if let cookieString = cookieHeaders["Cookie"] {
                CFHTTPMessageSetHeaderFieldValue(cfRequest, "Cookie" as CFString, cookieString as CFString)
            }
        }
        
        // Create read stream for HTTP request
        let readStream = CFReadStreamCreateForHTTPRequest(kCFAllocatorDefault, cfRequest).takeRetainedValue()
        
        inputStream = readStream as InputStream
        
        // 关键：设置 kCFStreamSSLPeerName 为原始域名，这样 TLS SNI 就会使用正确的主机名！
        if let host = request.allHTTPHeaderFields?["Host"] ?? originalHost {
            let sslSettings: [String: Any] = [
                kCFStreamSSLPeerName as String: host
            ]
            inputStream?.setProperty(sslSettings, forKey: Stream.PropertyKey(kCFStreamPropertySSLSettings as String))
            NSLog("[NitroDns] TLS SNI set to: %@", host)
        }
        
        inputStream?.delegate = self
        
        currentRunLoop = RunLoop.current
        if let mode = RunLoop.current.currentMode, mode != .common {
            runLoopMode = mode
        }
        
        inputStream?.schedule(in: currentRunLoop!, forMode: runLoopMode)
        inputStream?.open()
    }
    
    private func resolveDNS(host: String) -> String? {
        return autoreleasepool {
            guard let res = dns_lookup_sync(host, 0).data else { return nil }
            let data = String(cString: res)
            if let array = try? JSONSerialization.jsonObject(with: data.data(using: .utf8)!) as? [String],
               let first = array.first {
                return first
            }
            return nil
        }
    }
    
    public override func stopLoading() {
        if let stream = inputStream, stream.streamStatus == .open {
            stream.remove(from: currentRunLoop ?? .current, forMode: runLoopMode)
            stream.delegate = nil
            stream.close()
        }
        inputStream = nil
    }
    
    // MARK: - StreamDelegate
    
    public func stream(_ aStream: Stream, handle eventCode: Stream.Event) {
        switch eventCode {
        case .hasBytesAvailable:
            handleBytesAvailable(aStream)
            
        case .errorOccurred:
            handleError(aStream)
            
        case .endEncountered:
            handleEndEncountered(aStream)
            
        default:
            break
        }
    }
    
    private func handleBytesAvailable(_ aStream: Stream) {
        guard let readStream = aStream as? InputStream else { return }
        
        // Get HTTP response header from stream
        let cfReadStream = readStream as CFReadStream
        let propertyKey = CFStreamPropertyKey(rawValue: kCFStreamPropertyHTTPResponseHeader)
        guard let message = CFReadStreamCopyProperty(cfReadStream, propertyKey) else {
            return
        }
        
        let httpMessage = message as! CFHTTPMessage
        
        if CFHTTPMessageIsHeaderComplete(httpMessage) {
            // Read data
            var buffer = [UInt8](repeating: 0, count: 16 * 1024)
            let bytesRead = readStream.read(&buffer, maxLength: buffer.count)
            
            if !headerComplete {
                headerComplete = true
                
                // Get response headers
                let headersDict = CFHTTPMessageCopyAllHeaderFields(httpMessage)?.takeRetainedValue() as? [String: String] ?? [:]
                let httpVersion = CFHTTPMessageCopyVersion(httpMessage).takeRetainedValue() as String
                let statusCode = CFHTTPMessageGetResponseStatusCode(httpMessage)
                
                NSLog("[NitroDns] Response: %d, headers: %@", statusCode, headersDict)
                
                // --- Handle Redirect (3xx) ---
                if statusCode >= 300 && statusCode < 400, let location = headersDict["Location"] ?? headersDict["location"] {
                    NSLog("[NitroDns] Detected redirect to: %@", location)
                    if let redirectURL = URL(string: location, relativeTo: originalURL) {
                        // Create a redirect request
                        var redirectRequest = URLRequest(url: redirectURL)
                        // Copy relevant headers etc if needed, but system usually handles this
                        
                        // Notify client about redirect
                        if let response = HTTPURLResponse(url: originalURL!, statusCode: Int(statusCode), httpVersion: httpVersion, headerFields: headersDict) {
                            client?.urlProtocol(self, wasRedirectedTo: redirectRequest, redirectResponse: response)
                        }
                        
                        // IMPORTANT: Stop current loading and return
                        stopLoading()
                        return
                    }
                }
                
                // Create HTTPURLResponse with ORIGINAL URL
                if let originalURL = originalURL {
                    let response = HTTPURLResponse(
                        url: originalURL,
                        statusCode: Int(statusCode),
                        httpVersion: httpVersion,
                        headerFields: headersDict
                    )
                    
                    if let response = response {
                        client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
                    }
                }
            }
            
            // Deliver data
            if bytesRead > 0 {
                let data = Data(bytes: buffer, count: bytesRead)
                client?.urlProtocol(self, didLoad: data)
            }
        }
    }
    
    private func handleError(_ aStream: Stream) {
        aStream.remove(from: currentRunLoop ?? .current, forMode: runLoopMode)
        aStream.delegate = nil
        aStream.close()
        
        // 如果 IP 请求失败，尝试使用原始域名重试
        if currentRequest?.url?.host != originalHost, let domainRequest = requestWithDomain {
            NSLog("[NitroDns] IP request failed, retrying with domain")
            requestWithIP = nil
            headerComplete = false
            startRequest(domainRequest)
        } else {
            let error = aStream.streamError ?? NSError(domain: "NitroDns", code: -4, userInfo: [NSLocalizedDescriptionKey: "Stream error"])
            NSLog("[NitroDns] Request failed: %@", error.localizedDescription)
            client?.urlProtocol(self, didFailWithError: error)
        }
    }
    
    private func handleEndEncountered(_ aStream: Stream) {
        NSLog("[NitroDns] Request completed successfully")
        
        // Handle cookies from response
        if let readStream = aStream as? InputStream,
           let originalURL = originalURL {
            let cfReadStream = readStream as CFReadStream
            let propertyKey = CFStreamPropertyKey(rawValue: kCFStreamPropertyHTTPResponseHeader)
            if let message = CFReadStreamCopyProperty(cfReadStream, propertyKey) {
                let httpMessage = message as! CFHTTPMessage
                if let headersDict = CFHTTPMessageCopyAllHeaderFields(httpMessage)?.takeRetainedValue() as? [String: String] {
                    let cookies = HTTPCookie.cookies(withResponseHeaderFields: headersDict, for: originalURL)
                    for cookie in cookies {
                        HTTPCookieStorage.shared.setCookie(cookie)
                    }
                }
            }
        }
        
        aStream.remove(from: currentRunLoop ?? .current, forMode: runLoopMode)
        aStream.delegate = nil
        aStream.close()
        
        client?.urlProtocolDidFinishLoading(self)
    }
}
