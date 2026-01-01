#pragma once

#include "CachePolicy.hpp"
#include "HybridNitroDnsSpec.hpp"
#include <NitroModules/Promise.hpp>
#include <memory>
#include <optional>
#include <string>

namespace margelo {
namespace nitro {
namespace nitro_dns {

class HybridResolver : public HybridNitroResolverSpec {
public:
  explicit HybridResolver(int32_t id) : HybridObject(TAG), id_(id) {}
  ~HybridResolver() override;

  void cancel() override;
  std::string getServers() override;
  void setServers(const std::string &servers) override;
  std::shared_ptr<Promise<std::string>>
  resolveipv4(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveipv6(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveMx(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveTxt(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveBase64Txt(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveCname(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveNs(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveSoa(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveSrv(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveCaa(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveNaptr(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolvePtr(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveTlsa(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveAny(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>> reverse(const std::string &ip) override;

  void setLocalAddress(const std::string &v4, const std::string &v6) override;
  void clearCache() override;

private:
  int32_t id_;
};

class HybridDns : public HybridNitroDnsSpec {
public:
  HybridDns() : HybridObject(TAG), HybridNitroDnsSpec() {}

  std::shared_ptr<HybridNitroResolverSpec>
  createResolver(const std::optional<std::string> &config) override;
  std::string getServers() override;
  void setServers(const std::string &servers) override;
  std::string lookup(const std::string &hostname, double family) override;

  std::shared_ptr<Promise<std::string>>
  resolve4(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolve6(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveMx(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveTxt(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveCname(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveNs(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveSoa(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveSrv(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveCaa(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveNaptr(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolvePtr(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveTlsa(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>>
  resolveAny(const std::string &hostname) override;
  std::shared_ptr<Promise<std::string>> reverse(const std::string &ip) override;
  std::shared_ptr<Promise<std::string>>
  lookupService(const std::string &address, double port) override;
  void setNativeInterceptionEnabled(bool enabled) override;
  void setVerbose(bool enabled) override;
  void clearCache() override;
  void setCacheSize(double size) override;
  void setCachePolicy(CachePolicy policy, double staleTtl) override;
};

} // namespace nitro_dns
} // namespace nitro
} // namespace margelo
