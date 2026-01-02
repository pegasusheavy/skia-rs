import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'app-home',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <div class="home-page">
      <!-- Hero Section -->
      <section class="hero animate-fade-in">
        <div class="hero-badge badge badge-accent">
          <span class="pulse-dot"></span>
          Pure Rust Implementation
        </div>

        <h1 class="hero-title">
          <span class="gradient-text">skia-rs</span>
        </h1>

        <p class="hero-subtitle">
          A 100% pure Rust implementation of Google's Skia 2D graphics library,
          providing complete API compatibility and C FFI bindings for cross-platform graphics.
        </p>

        <div class="hero-actions">
          <a routerLink="/quick-start" class="btn-primary">
            Get Started
            <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="5" y1="12" x2="19" y2="12"></line>
              <polyline points="12 5 19 12 12 19"></polyline>
            </svg>
          </a>
          <a href="https://github.com/example/skia-rs" target="_blank" class="btn-secondary">
            <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
            </svg>
            View on GitHub
          </a>
        </div>

        <!-- Code preview -->
        <div class="code-preview animate-slide-up delay-200">
          <div class="code-header">
            <span class="code-dot red"></span>
            <span class="code-dot yellow"></span>
            <span class="code-dot green"></span>
            <span class="code-filename">main.rs</span>
          </div>
          <pre class="code-content"><code class="language-rust" [innerHTML]="heroCode"></code></pre>
        </div>
      </section>

      <!-- Features Section -->
      <section class="features animate-slide-up delay-300">
        <h2 class="section-title">Why skia-rs?</h2>

        <div class="features-grid">
          <div class="feature-card card hover-lift">
            <div class="feature-icon">🦀</div>
            <h3 class="feature-title">100% Pure Rust</h3>
            <p class="feature-desc">
              No C++ dependencies. Built entirely in Rust with memory safety guarantees
              and zero-cost abstractions.
            </p>
          </div>

          <div class="feature-card card hover-lift">
            <div class="feature-icon">🔄</div>
            <h3 class="feature-title">API Compatible</h3>
            <p class="feature-desc">
              Drop-in replacement for Skia. Same API structure makes migration seamless
              for existing projects.
            </p>
          </div>

          <div class="feature-card card hover-lift">
            <div class="feature-icon">🔗</div>
            <h3 class="feature-title">C FFI Bindings</h3>
            <p class="feature-desc">
              Full C API bindings for cross-language interoperability. Use from C, C++,
              Python, and more.
            </p>
          </div>

          <div class="feature-card card hover-lift">
            <div class="feature-icon">⚡</div>
            <h3 class="feature-title">High Performance</h3>
            <p class="feature-desc">
              Optimized software rasterizer with SIMD support. GPU backends via wgpu
              for hardware acceleration.
            </p>
          </div>

          <div class="feature-card card hover-lift">
            <div class="feature-icon">📦</div>
            <h3 class="feature-title">Modular Crates</h3>
            <p class="feature-desc">
              Pick only what you need. 12 focused crates for core, paths, paint,
              canvas, text, codecs, and more.
            </p>
          </div>

          <div class="feature-card card hover-lift">
            <div class="feature-icon">🎨</div>
            <h3 class="feature-title">Rich Graphics</h3>
            <p class="feature-desc">
              Full 2D graphics suite: paths, gradients, shaders, filters, text rendering,
              image codecs, SVG, and PDF.
            </p>
          </div>
        </div>
      </section>

      <!-- Quick Links Section -->
      <section class="quick-links animate-slide-up delay-400">
        <h2 class="section-title">Quick Links</h2>

        <div class="links-grid">
          <a routerLink="/installation" class="link-card card hover-lift">
            <div class="link-icon">📦</div>
            <div class="link-content">
              <h3>Installation</h3>
              <p>Add skia-rs to your Cargo.toml</p>
            </div>
            <svg class="link-arrow" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="5" y1="12" x2="19" y2="12"></line>
              <polyline points="12 5 19 12 12 19"></polyline>
            </svg>
          </a>

          <a routerLink="/api/core" class="link-card card hover-lift">
            <div class="link-icon">📚</div>
            <div class="link-content">
              <h3>API Reference</h3>
              <p>Explore the full API documentation</p>
            </div>
            <svg class="link-arrow" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="5" y1="12" x2="19" y2="12"></line>
              <polyline points="12 5 19 12 12 19"></polyline>
            </svg>
          </a>

          <a routerLink="/examples/basic-drawing" class="link-card card hover-lift">
            <div class="link-icon">🎯</div>
            <div class="link-content">
              <h3>Examples</h3>
              <p>Learn from working code samples</p>
            </div>
            <svg class="link-arrow" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="5" y1="12" x2="19" y2="12"></line>
              <polyline points="12 5 19 12 12 19"></polyline>
            </svg>
          </a>

          <a routerLink="/benchmarks" class="link-card card hover-lift">
            <div class="link-icon">⚡</div>
            <div class="link-content">
              <h3>Benchmarks</h3>
              <p>Performance comparisons and metrics</p>
            </div>
            <svg class="link-arrow" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="5" y1="12" x2="19" y2="12"></line>
              <polyline points="12 5 19 12 12 19"></polyline>
            </svg>
          </a>
        </div>
      </section>

      <!-- Stats Section -->
      <section class="stats animate-slide-up delay-500">
        <div class="stats-grid">
          <div class="stat-item">
            <div class="stat-value gradient-text">12</div>
            <div class="stat-label">Crates</div>
          </div>
          <div class="stat-item">
            <div class="stat-value gradient-text">100%</div>
            <div class="stat-label">Pure Rust</div>
          </div>
          <div class="stat-item">
            <div class="stat-value gradient-text">47x</div>
            <div class="stat-label">Faster Fills</div>
          </div>
          <div class="stat-item">
            <div class="stat-value gradient-text">17</div>
            <div class="stat-label">Fuzz Targets</div>
          </div>
        </div>
      </section>
    </div>
  `,
  styles: [`
    .home-page {
      padding-bottom: 4rem;
    }

    /* Hero Section */
    .hero {
      text-align: center;
      padding: 3rem 0 4rem;
    }

    .hero-badge {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      margin-bottom: 1.5rem;
    }

    .pulse-dot {
      width: 6px;
      height: 6px;
      background: var(--color-accent-primary);
      border-radius: 50%;
      animation: pulse 2s ease-in-out infinite;
    }

    .hero-title {
      font-size: 4rem;
      font-weight: 800;
      margin-bottom: 1.5rem;
      letter-spacing: -0.02em;
    }

    .hero-subtitle {
      font-size: 1.25rem;
      color: var(--color-text-secondary);
      max-width: 600px;
      margin: 0 auto 2rem;
      line-height: 1.7;
    }

    .hero-actions {
      display: flex;
      justify-content: center;
      gap: 1rem;
      margin-bottom: 3rem;
    }

    .btn-primary, .btn-secondary {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
    }

    /* Code Preview */
    .code-preview {
      max-width: 700px;
      margin: 0 auto;
      border-radius: 12px;
      overflow: hidden;
      background: var(--color-bg-card);
      border: 1px solid var(--color-border);
      text-align: left;
    }

    .code-header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.75rem 1rem;
      background: var(--color-bg-tertiary);
      border-bottom: 1px solid var(--color-border);
    }

    .code-dot {
      width: 12px;
      height: 12px;
      border-radius: 50%;
    }

    .code-dot.red { background: #ff5f56; }
    .code-dot.yellow { background: #ffbd2e; }
    .code-dot.green { background: #27c93f; }

    .code-filename {
      margin-left: auto;
      font-family: var(--font-mono);
      font-size: 0.75rem;
      color: var(--color-text-muted);
    }

    .code-content {
      margin: 0;
      padding: 1.5rem;
      font-size: 0.875rem;
      line-height: 1.6;
      overflow-x: auto;
    }

    /* Features Section */
    .features {
      margin-top: 4rem;
    }

    .section-title {
      font-size: 1.75rem;
      font-weight: 700;
      margin-bottom: 2rem;
      text-align: center;
    }

    .features-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
      gap: 1.5rem;
    }

    .feature-card {
      text-align: center;
      padding: 2rem;
    }

    .feature-icon {
      font-size: 2.5rem;
      margin-bottom: 1rem;
    }

    .feature-title {
      font-size: 1.125rem;
      font-weight: 600;
      margin-bottom: 0.75rem;
    }

    .feature-desc {
      color: var(--color-text-secondary);
      font-size: 0.875rem;
      line-height: 1.6;
      margin: 0;
    }

    /* Quick Links Section */
    .quick-links {
      margin-top: 4rem;
    }

    .links-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
      gap: 1rem;
    }

    .link-card {
      display: flex;
      align-items: center;
      gap: 1rem;
      padding: 1.25rem 1.5rem;
      text-decoration: none;
      color: inherit;
    }

    .link-icon {
      font-size: 1.5rem;
    }

    .link-content {
      flex: 1;
    }

    .link-content h3 {
      font-size: 1rem;
      font-weight: 600;
      margin-bottom: 0.25rem;
    }

    .link-content p {
      font-size: 0.8125rem;
      color: var(--color-text-muted);
      margin: 0;
    }

    .link-arrow {
      color: var(--color-text-muted);
      transition: transform var(--transition-fast), color var(--transition-fast);
    }

    .link-card:hover .link-arrow {
      transform: translateX(4px);
      color: var(--color-accent-primary);
    }

    /* Stats Section */
    .stats {
      margin-top: 4rem;
      padding: 3rem 0;
      border-top: 1px solid var(--color-border);
      border-bottom: 1px solid var(--color-border);
    }

    .stats-grid {
      display: grid;
      grid-template-columns: repeat(4, 1fr);
      gap: 2rem;
      text-align: center;
    }

    .stat-value {
      font-size: 3rem;
      font-weight: 800;
      margin-bottom: 0.5rem;
    }

    .stat-label {
      font-size: 0.875rem;
      color: var(--color-text-muted);
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    @media (max-width: 768px) {
      .hero-title {
        font-size: 2.5rem;
      }

      .hero-subtitle {
        font-size: 1rem;
      }

      .hero-actions {
        flex-direction: column;
        align-items: center;
      }

      .stats-grid {
        grid-template-columns: repeat(2, 1fr);
      }

      .stat-value {
        font-size: 2rem;
      }
    }
  `]
})
export class HomeComponent {
  heroCode = `use skia_rs::{Surface, Paint, Color, Path};

fn main() {
    // Create a surface
    let mut surface = Surface::new_raster_n32_premul(800, 600).unwrap();
    let canvas = surface.canvas();

    // Draw a gradient background
    canvas.clear(Color::from_rgb(18, 18, 26));

    // Create a path
    let mut path = Path::new();
    path.move_to(100.0, 100.0)
        .cubic_to(200.0, 50.0, 300.0, 150.0, 400.0, 100.0);

    // Draw with anti-aliased paint
    let mut paint = Paint::new();
    paint.set_anti_alias(true)
         .set_color(Color::from_rgb(255, 107, 53))
         .set_stroke_width(4.0);

    canvas.draw_path(&amp;path, &amp;paint);
}`;
}
