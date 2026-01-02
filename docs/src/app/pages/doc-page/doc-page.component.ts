import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterLink } from '@angular/router';

@Component({
  selector: 'app-doc-page',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <article class="doc-page animate-fade-in">
      <header class="doc-header">
        <div class="breadcrumb">
          <a routerLink="/">Home</a>
          <span class="separator">/</span>
          <span class="current">{{ title }}</span>
        </div>
        <h1>{{ title }}</h1>
        <p class="lead" *ngIf="description">{{ description }}</p>
      </header>

      <section class="doc-content">
        <ng-container *ngIf="pageId === 'installation'">
          <h2>Adding skia-rs to your project</h2>
          <p>Add the following to your <code>Cargo.toml</code>:</p>
          <pre class="code-block"><code [innerHTML]="installCode"></code></pre>

          <h3>Feature Flags</h3>
          <p>Enable optional features as needed:</p>
          <pre class="code-block"><code [innerHTML]="featureCode"></code></pre>
        </ng-container>

        <ng-container *ngIf="pageId === 'quick-start'">
          <h2>Your first skia-rs program</h2>
          <p>Let's create a simple program that draws a rectangle:</p>
          <pre class="code-block"><code [innerHTML]="quickStartCode"></code></pre>

          <h3>Running the example</h3>
          <pre class="code-block"><code>cargo run</code></pre>
        </ng-container>

        <ng-container *ngIf="pageId === 'benchmarks'">
          <h2>Performance Benchmarks</h2>
          <p>skia-rs includes a comprehensive benchmark suite.</p>

          <div class="benchmark-item">
            <div class="benchmark-name">Rectangle Fill (1000x1000)</div>
            <div class="benchmark-bar"><div class="benchmark-fill" style="width: 98%"></div></div>
            <div class="benchmark-value"><span class="improvement">68x faster</span></div>
          </div>
        </ng-container>

        <ng-container *ngIf="pageId === 'changelog'">
          <h2>Changelog</h2>
          <div class="changelog-entry">
            <span class="version">v0.1.0</span> - Initial Release
          </div>
        </ng-container>

        <ng-container *ngIf="section === 'api'">
          <span class="crate-badge">{{ crate }}</span>
          <p>Documentation for <code>{{ crate }}</code>.</p>
        </ng-container>

        <ng-container *ngIf="section === 'examples'">
          <h2>{{ title }}</h2>
          <p>Example demonstrating {{ slug }}.</p>
        </ng-container>

        <ng-container *ngIf="showPlaceholder">
          <div class="placeholder">
            <h2>Coming Soon</h2>
            <p>This page is under construction.</p>
          </div>
        </ng-container>
      </section>
    </article>
  `,
  styles: [`
    .doc-page { max-width: 800px; }
    .doc-header { margin-bottom: 2rem; padding-bottom: 1.5rem; border-bottom: 1px solid var(--color-border); }
    .breadcrumb { display: flex; gap: 0.5rem; margin-bottom: 1rem; font-size: 0.875rem; }
    .breadcrumb a { color: var(--color-text-muted); }
    .doc-header h1 { font-size: 2.25rem; font-weight: 700; }
    .lead { font-size: 1.125rem; color: var(--color-text-secondary); }
    .doc-content h2 { font-size: 1.5rem; margin: 2rem 0 1rem; }
    .doc-content h3 { font-size: 1.25rem; margin: 1.5rem 0 0.75rem; }
    .code-block { background: var(--color-bg-card); border: 1px solid var(--color-border); border-radius: 8px; padding: 1.25rem; overflow-x: auto; margin: 1rem 0; }
    .crate-badge { padding: 0.25rem 0.75rem; background: var(--gradient-primary); color: white; font-family: var(--font-mono); border-radius: 6px; }
    .benchmark-item { margin-bottom: 1.25rem; }
    .benchmark-name { font-weight: 500; margin-bottom: 0.5rem; }
    .benchmark-bar { height: 8px; background: var(--color-bg-tertiary); border-radius: 4px; }
    .benchmark-fill { height: 100%; background: var(--gradient-primary); border-radius: 4px; }
    .benchmark-value { font-size: 0.8125rem; color: var(--color-text-muted); }
    .improvement { color: #22c55e; font-weight: 600; }
    .changelog-entry { margin-bottom: 1rem; }
    .version { font-family: var(--font-mono); color: var(--color-accent-primary); font-weight: 700; }
    .placeholder { text-align: center; padding: 4rem; }
  `]
})
export class DocPageComponent implements OnInit {
  title = '';
  description = '';
  section = '';
  crate = '';
  item = '';
  slug = '';
  pageId = '';

  // Code snippets (to avoid Angular template parsing issues with double braces)
  quickStartCode = `use skia_rs_canvas::Surface;
use skia_rs_core::{Color, Rect};
use skia_rs_paint::{Paint, Style};

fn main() {
    // Create a 400x300 RGBA surface
    let mut surface = Surface::new_raster_n32_premul(400, 300)
        .expect("Failed to create surface");

    // Get a canvas to draw on
    let mut canvas = surface.raster_canvas();

    // Clear with a dark background
    canvas.clear(Color::from_rgb(18, 18, 26));

    // Create a paint with an orange color
    let mut paint = Paint::new();
    paint.set_anti_alias(true);
    paint.set_color32(Color::from_rgb(255, 107, 53));
    paint.set_style(Style::Fill);

    // Draw a rectangle
    let rect = Rect::from_xywh(50.0, 50.0, 300.0, 200.0);
    canvas.draw_rect(&amp;rect, &amp;paint);

    // Save to a file (requires codecs feature)
    // surface.save_png("output.png");

    println!("Drawing complete!");
}`;

  featureFlagsCode = `[dependencies]
skia-rs = { version = "0.1", features = ["gpu", "svg", "pdf"] }`;

  exampleCode = `// Example code will be loaded here
use skia_rs::*;

fn main() {
    // Your example code
}`;

  installCode = `[dependencies]
skia-rs = "0.1"

# Or pick specific crates:
skia-rs-core = "0.1"
skia-rs-path = "0.1"`;

  featureCode = `skia-rs = { version = "0.1", features = ["gpu", "svg", "pdf"] }`;

  get showPlaceholder(): boolean {
    return !this.pageId && this.section !== 'api' && this.section !== 'examples';
  }

  constructor(private route: ActivatedRoute) {}

  ngOnInit(): void {
    this.route.data.subscribe(data => {
      this.title = data['title'] || '';
      this.section = data['section'] || '';
    });

    this.route.params.subscribe(params => {
      this.crate = params['crate'] || '';
      this.item = params['item'] || '';
      this.slug = params['slug'] || '';

      const path = this.route.snapshot.routeConfig?.path || '';
      if (path === 'installation') this.pageId = 'installation';
      else if (path === 'quick-start') this.pageId = 'quick-start';
      else if (path === 'changelog') this.pageId = 'changelog';
      else if (path === 'benchmarks') this.pageId = 'benchmarks';

      if (!this.title) {
        if (this.crate) this.title = this.item || this.crate;
        else if (this.slug) this.title = this.slug.split('-').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');
      }
    });
  }
}
