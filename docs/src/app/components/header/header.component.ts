import { Component, EventEmitter, Input, Output } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'app-header',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <header class="header glass">
      <div class="header-content">
        <!-- Left section -->
        <div class="header-left">
          <button class="menu-btn" (click)="menuToggle.emit()" aria-label="Toggle menu">
            <svg 
              xmlns="http://www.w3.org/2000/svg" 
              width="24" 
              height="24" 
              viewBox="0 0 24 24" 
              fill="none" 
              stroke="currentColor" 
              stroke-width="2" 
              stroke-linecap="round" 
              stroke-linejoin="round">
              <line *ngIf="!sidebarOpen" x1="3" y1="6" x2="21" y2="6"></line>
              <line *ngIf="!sidebarOpen" x1="3" y1="12" x2="21" y2="12"></line>
              <line *ngIf="!sidebarOpen" x1="3" y1="18" x2="21" y2="18"></line>
              <line *ngIf="sidebarOpen" x1="18" y1="6" x2="6" y2="18"></line>
              <line *ngIf="sidebarOpen" x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
          
          <a routerLink="/" class="logo">
            <div class="logo-icon">
              <svg width="32" height="32" viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
                <defs>
                  <linearGradient id="logoGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" style="stop-color:#ff6b35"/>
                    <stop offset="100%" style="stop-color:#f7c94b"/>
                  </linearGradient>
                </defs>
                <rect x="4" y="4" width="24" height="24" rx="4" fill="url(#logoGrad)"/>
                <path d="M10 12h12M10 16h8M10 20h10" stroke="white" stroke-width="2" stroke-linecap="round"/>
              </svg>
            </div>
            <span class="logo-text">
              <span class="gradient-text">skia-rs</span>
              <span class="logo-docs">docs</span>
            </span>
          </a>
        </div>
        
        <!-- Center - Search -->
        <div class="header-center">
          <div class="search-container">
            <svg class="search-icon" xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="11" cy="11" r="8"></circle>
              <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
            </svg>
            <input 
              type="text" 
              placeholder="Search documentation..." 
              class="search-input"
              (focus)="searchFocused = true"
              (blur)="searchFocused = false">
            <kbd class="search-shortcut">⌘K</kbd>
          </div>
        </div>
        
        <!-- Right section -->
        <div class="header-right">
          <a href="https://github.com/example/skia-rs" target="_blank" rel="noopener" class="header-link" title="GitHub">
            <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
            </svg>
          </a>
          
          <a href="https://crates.io/crates/skia-rs" target="_blank" rel="noopener" class="header-link crates-link" title="crates.io">
            <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 512 512" fill="currentColor">
              <path d="M239.1 6.3l-208 78c-18.7 7-31.1 25-31.1 45v225.1c0 18.2 10.3 34.8 26.5 42.9l208 104c13.5 6.8 29.4 6.8 42.9 0l208-104c16.3-8.1 26.5-24.8 26.5-42.9V129.3c0-20-12.4-37.9-31.1-44.9l-208-78C262 2.2 250 2.2 239.1 6.3zM256 68.4l192 72v1.1l-192 78-192-78v-1.1l192-72zm32 356V275.5l160-65v160.4l-160 53.5z"/>
            </svg>
          </a>
          
          <span class="version-badge badge badge-accent">v0.1.0</span>
        </div>
      </div>
    </header>
  `,
  styles: [`
    .header {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      height: var(--header-height);
      z-index: 50;
      border-bottom: 1px solid var(--color-border);
    }
    
    .header-content {
      height: 100%;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0 1.5rem;
      gap: 1rem;
    }
    
    .header-left {
      display: flex;
      align-items: center;
      gap: 1rem;
    }
    
    .menu-btn {
      display: none;
      align-items: center;
      justify-content: center;
      width: 40px;
      height: 40px;
      background: transparent;
      border: none;
      color: var(--color-text-secondary);
      cursor: pointer;
      border-radius: 8px;
      transition: all var(--transition-fast);
    }
    
    .menu-btn:hover {
      background: var(--color-bg-tertiary);
      color: var(--color-text-primary);
    }
    
    .logo {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      text-decoration: none;
    }
    
    .logo-icon {
      display: flex;
      align-items: center;
    }
    
    .logo-text {
      display: flex;
      align-items: baseline;
      gap: 0.25rem;
      font-weight: 700;
      font-size: 1.25rem;
    }
    
    .logo-docs {
      color: var(--color-text-muted);
      font-weight: 400;
      font-size: 0.875rem;
    }
    
    .header-center {
      flex: 1;
      max-width: 500px;
      margin: 0 2rem;
    }
    
    .search-container {
      position: relative;
      display: flex;
      align-items: center;
    }
    
    .search-icon {
      position: absolute;
      left: 1rem;
      color: var(--color-text-muted);
      pointer-events: none;
    }
    
    .search-input {
      width: 100%;
      height: 42px;
      padding: 0 3rem;
      background: var(--color-bg-tertiary);
      border: 1px solid var(--color-border);
      border-radius: 10px;
      color: var(--color-text-primary);
      font-size: 0.875rem;
      transition: all var(--transition-fast);
    }
    
    .search-input::placeholder {
      color: var(--color-text-muted);
    }
    
    .search-input:focus {
      outline: none;
      border-color: var(--color-accent-primary);
      box-shadow: 0 0 0 3px rgba(255, 107, 53, 0.15);
    }
    
    .search-shortcut {
      position: absolute;
      right: 0.75rem;
      padding: 0.25rem 0.5rem;
      background: var(--color-bg-secondary);
      border: 1px solid var(--color-border);
      border-radius: 4px;
      color: var(--color-text-muted);
      font-family: var(--font-mono);
      font-size: 0.75rem;
    }
    
    .header-right {
      display: flex;
      align-items: center;
      gap: 1rem;
    }
    
    .header-link {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 40px;
      height: 40px;
      color: var(--color-text-secondary);
      border-radius: 8px;
      transition: all var(--transition-fast);
    }
    
    .header-link:hover {
      background: var(--color-bg-tertiary);
      color: var(--color-text-primary);
    }
    
    .version-badge {
      font-family: var(--font-mono);
      font-size: 0.75rem;
    }
    
    @media (max-width: 1024px) {
      .menu-btn {
        display: flex;
      }
      
      .header-center {
        display: none;
      }
    }
    
    @media (max-width: 640px) {
      .header-content {
        padding: 0 1rem;
      }
      
      .logo-text {
        display: none;
      }
      
      .version-badge {
        display: none;
      }
    }
  `]
})
export class HeaderComponent {
  @Input() sidebarOpen = true;
  @Output() menuToggle = new EventEmitter<void>();
  
  searchFocused = false;
}
