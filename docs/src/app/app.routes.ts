import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    loadComponent: () => import('./pages/home/home.component').then(m => m.HomeComponent),
  },
  {
    path: 'introduction',
    loadComponent: () => import('./pages/introduction/introduction.component').then(m => m.IntroductionComponent),
  },
  {
    path: 'installation',
    loadComponent: () => import('./pages/doc-page/doc-page.component').then(m => m.DocPageComponent),
    data: { title: 'Installation', section: 'getting-started' }
  },
  {
    path: 'quick-start',
    loadComponent: () => import('./pages/doc-page/doc-page.component').then(m => m.DocPageComponent),
    data: { title: 'Quick Start', section: 'getting-started' }
  },
  {
    path: 'guides/:slug',
    loadComponent: () => import('./pages/doc-page/doc-page.component').then(m => m.DocPageComponent),
    data: { section: 'guides' }
  },
  {
    path: 'api/:crate',
    loadComponent: () => import('./pages/doc-page/doc-page.component').then(m => m.DocPageComponent),
    data: { section: 'api' }
  },
  {
    path: 'api/:crate/:item',
    loadComponent: () => import('./pages/doc-page/doc-page.component').then(m => m.DocPageComponent),
    data: { section: 'api' }
  },
  {
    path: 'examples/:slug',
    loadComponent: () => import('./pages/doc-page/doc-page.component').then(m => m.DocPageComponent),
    data: { section: 'examples' }
  },
  {
    path: 'changelog',
    loadComponent: () => import('./pages/doc-page/doc-page.component').then(m => m.DocPageComponent),
    data: { title: 'Changelog', section: 'resources' }
  },
  {
    path: 'benchmarks',
    loadComponent: () => import('./pages/doc-page/doc-page.component').then(m => m.DocPageComponent),
    data: { title: 'Benchmarks', section: 'resources' }
  },
  {
    path: '**',
    redirectTo: '',
    pathMatch: 'full'
  }
];
