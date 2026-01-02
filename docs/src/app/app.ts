import { Component } from '@angular/core';
import { RouterOutlet, RouterLink, RouterLinkActive } from '@angular/router';
import { CommonModule } from '@angular/common';
import { SidebarComponent } from './components/sidebar/sidebar.component';
import { HeaderComponent } from './components/header/header.component';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [CommonModule, RouterOutlet, RouterLink, RouterLinkActive, SidebarComponent, HeaderComponent],
  template: `
    <div class="app-layout">
      <!-- Header -->
      <app-header 
        (menuToggle)="sidebarOpen = !sidebarOpen"
        [sidebarOpen]="sidebarOpen">
      </app-header>
      
      <!-- Sidebar -->
      <app-sidebar 
        [isOpen]="sidebarOpen" 
        (closeSidebar)="sidebarOpen = false">
      </app-sidebar>
      
      <!-- Main Content -->
      <main class="main-content" [class.sidebar-open]="sidebarOpen">
        <div class="content-wrapper">
          <router-outlet></router-outlet>
        </div>
      </main>
      
      <!-- Mobile overlay -->
      <div 
        class="mobile-overlay"
        [class.active]="sidebarOpen"
        (click)="sidebarOpen = false">
      </div>
    </div>
  `,
  styles: [`
    .app-layout {
      min-height: 100vh;
      display: flex;
      flex-direction: column;
    }
    
    .main-content {
      flex: 1;
      margin-left: var(--sidebar-width);
      margin-top: var(--header-height);
      padding: 2rem;
      transition: margin-left var(--transition-normal);
      background: var(--color-bg-primary);
    }
    
    .content-wrapper {
      max-width: var(--content-max-width);
      margin: 0 auto;
    }
    
    .mobile-overlay {
      display: none;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.6);
      backdrop-filter: blur(4px);
      z-index: 40;
      opacity: 0;
      pointer-events: none;
      transition: opacity var(--transition-normal);
    }
    
    .mobile-overlay.active {
      opacity: 1;
      pointer-events: auto;
    }
    
    @media (max-width: 1024px) {
      .main-content {
        margin-left: 0;
      }
      
      .mobile-overlay {
        display: block;
      }
    }
    
    @media (max-width: 640px) {
      .main-content {
        padding: 1rem;
      }
    }
  `]
})
export class AppComponent {
  sidebarOpen = true;
}
