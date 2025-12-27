//
//  NitroDnsRCTBridge.m
//  NitroDns
//
//  Bridge to integrate NitroDnsURLProtocol with React Native's networking
//  stack.
//

#import <Foundation/Foundation.h>

#if __has_include(<React/RCTBridge.h>)
#import <React/RCTBridge.h>
#import <React/RCTHTTPRequestHandler.h>

@interface NitroDnsRCTBridge : NSObject
@end

@implementation NitroDnsRCTBridge

// Use load to inject our URLProtocol before React Native initializes networking
+ (void)load {
  static dispatch_once_t onceToken;
  dispatch_once(&onceToken, ^{
    RCTSetCustomNSURLSessionConfigurationProvider(
        ^NSURLSessionConfiguration *_Nonnull {
          NSURLSessionConfiguration *config =
              [NSURLSessionConfiguration defaultSessionConfiguration];

          // Conditionally include our URLProtocol
          // We need to check if interception is enabled
          Class protocolClass = NSClassFromString(@"NitroDnsURLProtocol");
          if (protocolClass) {
            // Our protocol will be at the front, so it gets first chance
            NSMutableArray *protocols =
                [NSMutableArray arrayWithArray:config.protocolClasses];
            [protocols insertObject:protocolClass atIndex:0];
            config.protocolClasses = protocols;
            NSLog(@"[NitroDns] Custom URLProtocol injected into React Native's "
                  @"URLSessionConfiguration");
          } else {
            NSLog(@"[NitroDns] Warning: NitroDnsURLProtocol class not found");
          }

          return config;
        });
    NSLog(
        @"[NitroDns] RCTSetCustomNSURLSessionConfigurationProvider registered");
  });
}

@end

#else
// React-Core not available, skip RCT bridge
#endif
