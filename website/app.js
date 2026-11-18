'use strict';
document.body.classList.add('js-enabled');

const menu = document.querySelector('.menu-toggle');
const nav = document.querySelector('#main-nav');
if (menu && nav) {
  menu.hidden = false;
  const closeMenu = () => {
    menu.setAttribute('aria-expanded', 'false');
    nav.dataset.open = 'false';
  };
  menu.addEventListener('click', () => {
    const open = menu.getAttribute('aria-expanded') !== 'true';
    menu.setAttribute('aria-expanded', String(open));
    nav.dataset.open = String(open);
  });
  nav.addEventListener('click', (event) => {
    if (event.target.closest('a')) closeMenu();
  });
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && menu.getAttribute('aria-expanded') === 'true') {
      closeMenu();
      menu.focus();
    }
  });
  const wideScreen = window.matchMedia('(min-width: 761px)');
  wideScreen.addEventListener('change', (event) => {
    if (event.matches) closeMenu();
  });
}

const scenes = {
  alpine: {
    image: 'assets/cloud-alpine.png',
    alt: 'Actual Worldcraft application showing an alpine valley with conifers and mountain ridges.',
    label: 'Classical generation',
    title: 'Follow the ridgeline.',
    description: 'A seeded alpine landscape, ready to explore on foot or in flight.',
    counter: '01 / 03'
  },
  coast: {
    image: 'assets/cloud-coast.png',
    alt: 'Actual Worldcraft application showing a tree-covered coastal island surrounded by water.',
    label: 'Classical generation',
    title: 'Find a different shore.',
    description: 'A coastal preset with procedural nature and an open horizon.',
    counter: '02 / 03'
  },
  learned: {
    image: 'assets/alpha2-detailed-linux.png',
    alt: 'Actual Worldcraft application showing the saved detailed AI highlands example with trees and rolling terrain.',
    label: 'Optional AI terrain · finite regional bake',
    title: 'Let the highlands unfold.',
    description: 'The bundled detailed AI example. Explore saved relief without downloading models or running inference.',
    counter: '03 / 03'
  }
};

const tabs = Array.from(document.querySelectorAll('[data-scene]'));
const panel = document.querySelector('#landscape-panel');
const sceneImage = document.querySelector('#scene-image');
function selectScene(tab) {
  const scene = scenes[tab.dataset.scene];
  if (!scene || !panel || !sceneImage) return;
  for (const other of tabs) {
    const selected = other === tab;
    other.setAttribute('aria-selected', String(selected));
    other.tabIndex = selected ? 0 : -1;
  }
  panel.setAttribute('aria-labelledby', tab.id);
  sceneImage.src = scene.image;
  sceneImage.alt = scene.alt;
  document.querySelector('#scene-label').textContent = scene.label;
  document.querySelector('#scene-title').textContent = scene.title;
  document.querySelector('#scene-description').textContent = scene.description;
  document.querySelector('#scene-counter').textContent = scene.counter;
}
for (const tab of tabs) {
  tab.addEventListener('click', () => selectScene(tab));
  tab.addEventListener('keydown', (event) => {
    let index = tabs.indexOf(tab);
    if (event.key === 'ArrowRight') index = (index + 1) % tabs.length;
    else if (event.key === 'ArrowLeft') index = (index + tabs.length - 1) % tabs.length;
    else if (event.key === 'Home') index = 0;
    else if (event.key === 'End') index = tabs.length - 1;
    else return;
    event.preventDefault();
    tabs[index].focus();
    selectScene(tabs[index]);
  });
}
