#pragma once

#include "render/core/cyberpunk_effect.h"

// Per-pass variation stays fixed while the sweep moves smoothly downward.
namespace cyberpunk {
  inline constexpr int tooltipScanPeriodMs = 2400;
  inline float scanVariation(float seed, float salt) noexcept {
    const float value = std::sin(seed * 0.017F + salt * 12.9898F) * 43758.545F;
    return value - std::floor(value);
  }
  inline float scanDuration(float seed) noexcept { return 1600.0F + 600.0F * scanVariation(seed, 6); }
  inline float scanWidth(float seed) noexcept { return 0.09F + 0.09F * scanVariation(seed, 1); }
  inline int scanCount(float seed) noexcept { return 2 + std::min(2, static_cast<int>(scanVariation(seed, 7) * 3)); }
  inline float scanStripTop(float phase, int strip, float seed = 0) noexcept {
    const float cluster = static_cast<float>(scanCount(seed)) * scanWidth(seed);
    return std::clamp(phase, 0.0F, 1.0F) * (1.0F + cluster) - cluster + static_cast<float>(strip) * scanWidth(seed);
  }
  inline float scanOffset(float phase, int strip, float seed = 0) noexcept {
    const float amplitude = 4.0F + 6.0F * scanVariation(seed, 2);
    const float direction = scanVariation(seed, 3) < 0.5F ? -1.0F : 1.0F;
    const float speed = 1.0F + 0.8F * scanVariation(seed, 4);
    const float spacing = 0.45F + 0.35F * scanVariation(seed, 5);
    return direction * amplitude * std::sin(phase * speed + static_cast<float>(strip) * spacing) * envelope(phase);
  }
} // namespace cyberpunk
