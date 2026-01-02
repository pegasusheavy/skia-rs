import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'app-introduction',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <article class="doc-page animate-fade-in">
      <header class="doc-header">
        <div class="breadcrumb">
          <a routerLink="/">Home</a>
          <span class="separator">/</span>
          <span class="current">Introduction</span>
        </div>
        <h1>Introduction to skia-rs</h1>
        <p class="lead">
          A comprehensive guide to understanding skia-rs, a pure Rust implementation of
          Google's Skia 2D graphics library.
        </p>
      </header>
      
      <section class="doc-section">
        <h2 id="what-is-skia-rs">What is skia-rs?</h2>
        <p>
          <strong>skia-rs</strong> is a 100% pure Rust implementation of Google's Skia 2D graphics
          library. It provides a familiar API for developers who have worked with Skia while
          delivering the safety, performance, and ergonomics of Rust.
        </p>
        
        <div class="callout callout-info">
          <div class="callout-icon">💡</div>
          <div class="callout-content">
            <strong>Note:</strong> skia-rs aims for API compatibility with the original Skia library,
            making it easier to port existing code or follow Skia documentation.
          </div>
        </div>
      </section>
      
      <section class="doc-section">
        <h2 id="key-features">Key Features</h2>
        
        <div class="feature-list">
          <div class="feature-item">
            <h3>🦀 Pure Rust Implementation</h3>
            <p>
              No C++ dependencies or complex build requirements. skia-rs compiles with standard
              Rust tooling and works on all platforms that Rust supports.
            </p>
          </div>
          
          <div class="feature-item">
            <h3>🔄 API Compatibility</h3>
            <p>
              Designed to match Skia's API structure. If you know Skia, you'll feel right at home.
              Function names, parameter orders, and semantics align with the original.
            </p>
          </div>
          
          <div class="feature-item">
            <h3>📦 Modular Architecture</h3>
            <p>
              Pick only what you need. The library is split into focused crates:
            </p>
            <ul>
              <li><code>skia-rs-core</code> - Foundation types (Point, Rect, Color, Matrix)</li>
              <li><code>skia-rs-path</code> - Path geometry and operations</li>
              <li><code>skia-rs-paint</code> - Styling, shaders, and effects</li>
              <li><code>skia-rs-canvas</code> - Drawing surface and operations</li>
              <li><code>skia-rs-text</code> - Text layout and rendering</li>
              <li><code>skia-rs-codec</code> - Image encoding/decoding</li>
              <li><code>skia-rs-gpu</code> - GPU backends (wgpu)</li>
              <li>...and more</li>
            </ul>
          </div>
          
          <div class="feature-item">
            <h3>🔗 C FFI Bindings</h3>
            <p>
              Full C API through <code>skia-rs-ffi</code> for interoperability with C, C++,
              Python, Node.js, and other languages.
            </p>
          </div>
        </div>
      </section>
      
      <section class="doc-section">
        <h2 id="architecture">Architecture Overview</h2>
        
        <pre class="code-block"><code>skia-rs/
├── crates/
│   ├── skia-rs-core/     # Foundation types
│   ├── skia-rs-path/     # Path geometry
│   ├── skia-rs-paint/    # Styling & effects
│   ├── skia-rs-canvas/   # Drawing surface
│   ├── skia-rs-text/     # Typography
│   ├── skia-rs-gpu/      # GPU backends
│   ├── skia-rs-codec/    # Image I/O
│   ├── skia-rs-svg/      # SVG support
│   ├── skia-rs-pdf/      # PDF generation
│   ├── skia-rs-ffi/      # C bindings
│   └── skia-rs-safe/     # High-level API
└── ...</code></pre>
        
        <p>
          The crate dependency graph is carefully designed to minimize compile times
          and allow selective inclusion of features.
        </p>
      </section>
      
      <section class="doc-section">
        <h2 id="use-cases">Use Cases</h2>
        
        <div class="use-cases-grid">
          <div class="use-case card">
            <h4>🎮 Game UI</h4>
            <p>Render game interfaces, HUDs, and menus with hardware acceleration.</p>
          </div>
          <div class="use-case card">
            <h4>📊 Data Visualization</h4>
            <p>Create charts, graphs, and interactive visualizations.</p>
          </div>
          <div class="use-case card">
            <h4>🖼️ Image Processing</h4>
            <p>Apply filters, transformations, and effects to images.</p>
          </div>
          <div class="use-case card">
            <h4>📄 Document Generation</h4>
            <p>Generate PDFs, SVGs, and print-ready documents.</p>
          </div>
        </div>
      </section>
      
      <section class="doc-section">
        <h2 id="next-steps">Next Steps</h2>
        <p>Ready to get started? Here's where to go next:</p>
        
        <div class="next-links">
          <a routerLink="/installation" class="next-link">
            <span class="next-icon">📦</span>
            <span class="next-content">
              <strong>Installation</strong>
              <span>Add skia-rs to your project</span>
            </span>
          </a>
          <a routerLink="/quick-start" class="next-link">
            <span class="next-icon">🚀</span>
            <span class="next-content">
              <strong>Quick Start</strong>
              <span>Your first skia-rs program</span>
            </span>
          </a>
        </div>
      </section>
    </article>
  `,
  styles: [`
    .doc-page {
      max-width: 800px;
    }
    
    .doc-header {
      margin-bottom: 3rem;
      padding-bottom: 2rem;
      border-bottom: 1px solid var(--color-border);
    }
    
    .breadcrumb {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-bottom: 1rem;
      font-size: 0.875rem;
    }
    
    .breadcrumb a {
      color: var(--color-text-muted);
    }
    
    .breadcrumb a:hover {
      color: var(--color-accent-primary);
    }
    
    .breadcrumb .separator {
      color: var(--color-text-muted);
    }
    
    .breadcrumb .current {
      color: var(--color-text-secondary);
    }
    
    .doc-header h1 {
      font-size: 2.5rem;
      font-weight: 700;
      margin-bottom: 1rem;
    }
    
    .lead {
      font-size: 1.25rem;
      color: var(--color-text-secondary);
      line-height: 1.7;
      margin: 0;
    }
    
    .doc-section {
      margin-bottom: 3rem;
    }
    
    .doc-section h2 {
      font-size: 1.5rem;
      font-weight: 600;
      margin-bottom: 1rem;
      padding-top: 1rem;
    }
    
    .doc-section h3 {
      font-size: 1.125rem;
      font-weight: 600;
      margin-bottom: 0.5rem;
    }
    
    .doc-section p {
      line-height: 1.8;
    }
    
    .doc-section ul {
      padding-left: 1.5rem;
      margin: 1rem 0;
    }
    
    .doc-section li {
      margin-bottom: 0.5rem;
      color: var(--color-text-secondary);
    }
    
    .callout {
      display: flex;
      gap: 1rem;
      padding: 1rem 1.25rem;
      border-radius: 8px;
      margin: 1.5rem 0;
    }
    
    .callout-info {
      background: rgba(78, 205, 196, 0.1);
      border-left: 3px solid var(--color-accent-tertiary);
    }
    
    .callout-icon {
      font-size: 1.25rem;
    }
    
    .callout-content {
      flex: 1;
      font-size: 0.875rem;
      line-height: 1.6;
      color: var(--color-text-secondary);
    }
    
    .feature-list {
      display: flex;
      flex-direction: column;
      gap: 2rem;
      margin-top: 1.5rem;
    }
    
    .feature-item h3 {
      color: var(--color-text-primary);
    }
    
    .feature-item p {
      color: var(--color-text-secondary);
      margin-bottom: 0;
    }
    
    .code-block {
      background: var(--color-bg-card);
      border: 1px solid var(--color-border);
      border-radius: 8px;
      padding: 1.25rem;
      overflow-x: auto;
      font-size: 0.875rem;
    }
    
    .use-cases-grid {
      display: grid;
      grid-template-columns: repeat(2, 1fr);
      gap: 1rem;
      margin-top: 1.5rem;
    }
    
    .use-case {
      padding: 1.25rem;
    }
    
    .use-case h4 {
      font-size: 1rem;
      margin-bottom: 0.5rem;
    }
    
    .use-case p {
      font-size: 0.875rem;
      color: var(--color-text-muted);
      margin: 0;
    }
    
    .next-links {
      display: flex;
      gap: 1rem;
      margin-top: 1.5rem;
    }
    
    .next-link {
      display: flex;
      align-items: center;
      gap: 1rem;
      padding: 1rem 1.5rem;
      background: var(--color-bg-card);
      border: 1px solid var(--color-border);
      border-radius: 8px;
      text-decoration: none;
      color: inherit;
      flex: 1;
      transition: all var(--transition-fast);
    }
    
    .next-link:hover {
      border-color: var(--color-accent-primary);
      background: var(--color-bg-hover);
    }
    
    .next-icon {
      font-size: 1.5rem;
    }
    
    .next-content {
      display: flex;
      flex-direction: column;
    }
    
    .next-content strong {
      color: var(--color-text-primary);
    }
    
    .next-content span {
      font-size: 0.8125rem;
      color: var(--color-text-muted);
    }
    
    @media (max-width: 640px) {
      .use-cases-grid {
        grid-template-columns: 1fr;
      }
      
      .next-links {
        flex-direction: column;
      }
    }
  `]
})
export class IntroductionComponent {}
