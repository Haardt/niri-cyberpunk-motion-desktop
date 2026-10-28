#pragma once

#include <algorithm>
#include <cmath>

// A one-shot transition: no timer or frame work remains once it has settled.
namespace cyberpunk {
  struct Region {
    float x = 0, y = 0, width = 0, height = 0;
  };
  inline constexpr float durationMs = 320.0F;
  inline float progress(float elapsedMs) noexcept { return std::clamp(elapsedMs / durationMs, 0.0F, 1.0F); }
  inline float envelope(float phase) noexcept {
    if (phase <= 0.0F || phase >= 1.0F)
      return 0.0F;
    return std::sin(phase * 3.14159265358979323846F);
  }
} // namespace cyberpunk
