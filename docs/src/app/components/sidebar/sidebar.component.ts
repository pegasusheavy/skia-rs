import { Component, EventEmitter, Input, Output } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink, RouterLinkActive } from '@angular/router';

interface NavItem {
  label: string;
  path?: string;
  icon?: string;
  children?: NavItem[];
  badge?: string;
}

@Component({
  selector: 'app-sidebar',
  standalone: true,
  imports: [CommonModule, RouterLink, RouterLinkActive],
  template: `
    <aside class="sidebar glass" [class.open]="isOpen">
      <nav class="sidebar-nav">
        <!-- Getting Started -->
        <div class="nav-section">
          <h3 class="nav-section-title">Getting Started</h3>
          <ul class="nav-list">
            <li>
              <a routerLink="/introduction" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">📖</span>
                Introduction
              </a>
            </li>
            <li>
              <a routerLink="/installation" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">📦</span>
                Installation
              </a>
            </li>
            <li>
              <a routerLink="/quick-start" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">🚀</span>
                Quick Start
              </a>
            </li>
          </ul>
        </div>
        
        <!-- Core Concepts -->
        <div class="nav-section">
          <h3 class="nav-section-title">Core Concepts</h3>
          <ul class="nav-list">
            <li>
              <a routerLink="/guides/architecture" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">🏗️</span>
                Architecture
              </a>
            </li>
            <li>
              <a routerLink="/guides/geometry" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">📐</span>
                Geometry Types
              </a>
            </li>
            <li>
              <a routerLink="/guides/colors" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">🎨</span>
                Colors & Pixels
              </a>
            </li>
            <li>
              <a routerLink="/guides/matrix" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">🔢</span>
                Transformations
              </a>
            </li>
          </ul>
        </div>
        
        <!-- API Reference -->
        <div class="nav-section">
          <h3 class="nav-section-title">
            API Reference
            <span class="section-badge">12 crates</span>
          </h3>
          <ul class="nav-list">
            <li>
              <button class="nav-link expandable" (click)="toggleSection('core')">
                <span class="nav-icon">🔮</span>
                skia-rs-core
                <svg class="expand-icon" [class.expanded]="expandedSections['core']" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
              </button>
              <ul class="nav-sublist" *ngIf="expandedSections['core']">
                <li><a routerLink="/api/core/point" routerLinkActive="active" class="nav-sublink">Point</a></li>
                <li><a routerLink="/api/core/rect" routerLinkActive="active" class="nav-sublink">Rect</a></li>
                <li><a routerLink="/api/core/matrix" routerLinkActive="active" class="nav-sublink">Matrix</a></li>
                <li><a routerLink="/api/core/color" routerLinkActive="active" class="nav-sublink">Color</a></li>
                <li><a routerLink="/api/core/imageinfo" routerLinkActive="active" class="nav-sublink">ImageInfo</a></li>
              </ul>
            </li>
            <li>
              <button class="nav-link expandable" (click)="toggleSection('path')">
                <span class="nav-icon">✏️</span>
                skia-rs-path
                <svg class="expand-icon" [class.expanded]="expandedSections['path']" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
              </button>
              <ul class="nav-sublist" *ngIf="expandedSections['path']">
                <li><a routerLink="/api/path/path" routerLinkActive="active" class="nav-sublink">Path</a></li>
                <li><a routerLink="/api/path/builder" routerLinkActive="active" class="nav-sublink">PathBuilder</a></li>
                <li><a routerLink="/api/path/effects" routerLinkActive="active" class="nav-sublink">PathEffects</a></li>
                <li><a routerLink="/api/path/ops" routerLinkActive="active" class="nav-sublink">Boolean Ops</a></li>
              </ul>
            </li>
            <li>
              <button class="nav-link expandable" (click)="toggleSection('paint')">
                <span class="nav-icon">🖌️</span>
                skia-rs-paint
                <svg class="expand-icon" [class.expanded]="expandedSections['paint']" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
              </button>
              <ul class="nav-sublist" *ngIf="expandedSections['paint']">
                <li><a routerLink="/api/paint/paint" routerLinkActive="active" class="nav-sublink">Paint</a></li>
                <li><a routerLink="/api/paint/shaders" routerLinkActive="active" class="nav-sublink">Shaders</a></li>
                <li><a routerLink="/api/paint/filters" routerLinkActive="active" class="nav-sublink">Filters</a></li>
                <li><a routerLink="/api/paint/blend" routerLinkActive="active" class="nav-sublink">Blend Modes</a></li>
              </ul>
            </li>
            <li>
              <button class="nav-link expandable" (click)="toggleSection('canvas')">
                <span class="nav-icon">🖼️</span>
                skia-rs-canvas
                <svg class="expand-icon" [class.expanded]="expandedSections['canvas']" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
              </button>
              <ul class="nav-sublist" *ngIf="expandedSections['canvas']">
                <li><a routerLink="/api/canvas/canvas" routerLinkActive="active" class="nav-sublink">Canvas</a></li>
                <li><a routerLink="/api/canvas/surface" routerLinkActive="active" class="nav-sublink">Surface</a></li>
                <li><a routerLink="/api/canvas/picture" routerLinkActive="active" class="nav-sublink">Picture</a></li>
              </ul>
            </li>
            <li>
              <a routerLink="/api/text" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">📝</span>
                skia-rs-text
              </a>
            </li>
            <li>
              <a routerLink="/api/codec" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">🖼️</span>
                skia-rs-codec
              </a>
            </li>
            <li>
              <a routerLink="/api/gpu" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">🎮</span>
                skia-rs-gpu
                <span class="nav-badge">WIP</span>
              </a>
            </li>
            <li>
              <a routerLink="/api/ffi" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">🔗</span>
                skia-rs-ffi
              </a>
            </li>
          </ul>
        </div>
        
        <!-- Examples -->
        <div class="nav-section">
          <h3 class="nav-section-title">Examples</h3>
          <ul class="nav-list">
            <li>
              <a routerLink="/examples/basic-drawing" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">🎯</span>
                Basic Drawing
              </a>
            </li>
            <li>
              <a routerLink="/examples/paths" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">〰️</span>
                Path Operations
              </a>
            </li>
            <li>
              <a routerLink="/examples/gradients" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">🌈</span>
                Gradients & Shaders
              </a>
            </li>
            <li>
              <a routerLink="/examples/text-rendering" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">🔤</span>
                Text Rendering
              </a>
            </li>
            <li>
              <a routerLink="/examples/image-processing" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">📷</span>
                Image Processing
              </a>
            </li>
          </ul>
        </div>
        
        <!-- Resources -->
        <div class="nav-section">
          <h3 class="nav-section-title">Resources</h3>
          <ul class="nav-list">
            <li>
              <a routerLink="/changelog" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">📋</span>
                Changelog
              </a>
            </li>
            <li>
              <a routerLink="/benchmarks" routerLinkActive="active" class="nav-link">
                <span class="nav-icon">⚡</span>
                Benchmarks
              </a>
            </li>
            <li>
              <a href="https://github.com/example/skia-rs" target="_blank" class="nav-link external">
                <span class="nav-icon">💻</span>
                GitHub
                <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
                  <polyline points="15 3 21 3 21 9"></polyline>
                  <line x1="10" y1="14" x2="21" y2="3"></line>
                </svg>
              </a>
            </li>
          </ul>
        </div>
      </nav>
      
      <!-- Sidebar footer -->
      <div class="sidebar-footer">
        <div class="status-indicator">
          <span class="status-dot"></span>
          <span class="status-text">All systems operational</span>
        </div>
      </div>
    </aside>
  `,
  styles: [`
    .sidebar {
      position: fixed;
      top: var(--header-height);
      left: 0;
      bottom: 0;
      width: var(--sidebar-width);
      border-right: 1px solid var(--color-border);
      display: flex;
      flex-direction: column;
      z-index: 45;
      transform: translateX(0);
      transition: transform var(--transition-normal);
    }
    
    .sidebar-nav {
      flex: 1;
      overflow-y: auto;
      padding: 1.5rem 0;
    }
    
    .nav-section {
      margin-bottom: 1.5rem;
    }
    
    .nav-section-title {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0 1.5rem;
      margin-bottom: 0.5rem;
      font-size: 0.75rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--color-text-muted);
    }
    
    .section-badge {
      font-size: 0.65rem;
      font-weight: 500;
      text-transform: none;
      letter-spacing: normal;
      padding: 0.15rem 0.4rem;
      background: var(--color-bg-tertiary);
      border-radius: 4px;
      color: var(--color-text-secondary);
    }
    
    .nav-list {
      list-style: none;
      padding: 0;
      margin: 0;
    }
    
    .nav-link {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding: 0.625rem 1.5rem;
      color: var(--color-text-secondary);
      text-decoration: none;
      font-size: 0.875rem;
      font-weight: 500;
      transition: all var(--transition-fast);
      cursor: pointer;
      background: none;
      border: none;
      width: 100%;
      text-align: left;
    }
    
    .nav-link:hover {
      color: var(--color-text-primary);
      background: var(--color-bg-hover);
    }
    
    .nav-link.active {
      color: var(--color-accent-primary);
      background: rgba(255, 107, 53, 0.1);
      border-right: 2px solid var(--color-accent-primary);
    }
    
    .nav-icon {
      font-size: 1rem;
      width: 1.25rem;
      text-align: center;
    }
    
    .nav-badge {
      margin-left: auto;
      padding: 0.125rem 0.375rem;
      background: rgba(255, 107, 53, 0.15);
      color: var(--color-accent-primary);
      font-size: 0.625rem;
      font-weight: 600;
      border-radius: 4px;
      text-transform: uppercase;
    }
    
    .expandable {
      position: relative;
    }
    
    .expand-icon {
      margin-left: auto;
      transition: transform var(--transition-fast);
      color: var(--color-text-muted);
    }
    
    .expand-icon.expanded {
      transform: rotate(180deg);
    }
    
    .nav-sublist {
      list-style: none;
      padding: 0.25rem 0 0.5rem 0;
      margin: 0;
    }
    
    .nav-sublink {
      display: block;
      padding: 0.5rem 1.5rem 0.5rem 3.5rem;
      color: var(--color-text-muted);
      text-decoration: none;
      font-size: 0.8125rem;
      transition: all var(--transition-fast);
    }
    
    .nav-sublink:hover {
      color: var(--color-text-primary);
      background: var(--color-bg-hover);
    }
    
    .nav-sublink.active {
      color: var(--color-accent-primary);
    }
    
    .external svg {
      margin-left: auto;
      opacity: 0.5;
    }
    
    .sidebar-footer {
      padding: 1rem 1.5rem;
      border-top: 1px solid var(--color-border);
    }
    
    .status-indicator {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      font-size: 0.75rem;
      color: var(--color-text-muted);
    }
    
    .status-dot {
      width: 8px;
      height: 8px;
      background: #22c55e;
      border-radius: 50%;
      animation: pulse 2s ease-in-out infinite;
    }
    
    @keyframes pulse {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.5; }
    }
    
    @media (max-width: 1024px) {
      .sidebar {
        transform: translateX(-100%);
      }
      
      .sidebar.open {
        transform: translateX(0);
      }
    }
  `]
})
export class SidebarComponent {
  @Input() isOpen = true;
  @Output() closeSidebar = new EventEmitter<void>();
  
  expandedSections: Record<string, boolean> = {
    core: true,
    path: false,
    paint: false,
    canvas: false
  };
  
  toggleSection(section: string): void {
    this.expandedSections[section] = !this.expandedSections[section];
  }
}
