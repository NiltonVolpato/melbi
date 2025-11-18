// Tutorial navigation and content management

const tutorialState = {
  tutorials: [],
  currentStep: 0,
  dom: {
    title: null,
    body: null,
    tocList: null,
    tocDropdown: null,
    prevButton: null,
    nextButton: null,
  },
};

async function loadTutorials() {
  try {
    const response = await fetch('/tutorials.json');
    if (!response.ok) {
      throw new Error(`Failed to load tutorials: ${response.statusText}`);
    }
    tutorialState.tutorials = await response.json();
    return tutorialState.tutorials;
  } catch (err) {
    console.error('Error loading tutorials:', err);
    tutorialState.tutorials = [];
    return [];
  }
}

function getDomRefs() {
  return {
    title: document.getElementById('tutorial-title'),
    body: document.getElementById('tutorial-body'),
    tocList: document.getElementById('toc-list'),
    tocDropdown: document.getElementById('toc-dropdown'),
    prevButton: document.getElementById('prev-button'),
    nextButton: document.getElementById('next-button'),
  };
}

function toggleDropdown() {
  if (!tutorialState.dom.tocDropdown) return;
  tutorialState.dom.tocDropdown.classList.toggle('open');
}

function closeDropdown() {
  if (!tutorialState.dom.tocDropdown) return;
  tutorialState.dom.tocDropdown.classList.remove('open');
}

function renderTOC() {
  if (!tutorialState.dom.tocList) return;

  tutorialState.dom.tocList.innerHTML = '';

  tutorialState.tutorials.forEach((tutorial, index) => {
    const li = document.createElement('li');
    const button = document.createElement('button');
    button.textContent = tutorial.title;
    button.className = index === tutorialState.currentStep ? 'active' : '';
    button.addEventListener('click', () => {
      goToStep(index);
      closeDropdown();
    });
    li.appendChild(button);
    tutorialState.dom.tocList.appendChild(li);
  });
}

function updateNavigationButtons() {
  const { prevButton, nextButton } = tutorialState.dom;
  const total = tutorialState.tutorials.length;

  if (prevButton) {
    prevButton.disabled = tutorialState.currentStep === 0;
  }

  if (nextButton) {
    nextButton.disabled = tutorialState.currentStep >= total - 1;
  }
}

function renderMarkdown(markdown) {
  // Simple markdown rendering (you could use a library like marked.js for more features)
  let html = markdown;

  // Headers
  html = html.replace(/^### (.+)$/gm, '<h3>$1</h3>');
  html = html.replace(/^## (.+)$/gm, '<h2>$1</h2>');
  html = html.replace(/^# (.+)$/gm, '<h2>$1</h2>');

  // Code blocks
  html = html.replace(/```melbi\n([\s\S]*?)\n```/g, '<pre><code>$1</code></pre>');
  html = html.replace(/```\n([\s\S]*?)\n```/g, '<pre><code>$1</code></pre>');

  // Inline code
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>');

  // Bold
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');

  // Lists
  html = html.replace(/^- (.+)$/gm, '<li>$1</li>');
  html = html.replace(/(<li>.*<\/li>)/s, '<ul>$1</ul>');

  // Paragraphs
  html = html
    .split('\n\n')
    .map((para) => {
      para = para.trim();
      if (!para) return '';
      if (
        para.startsWith('<h') ||
        para.startsWith('<ul') ||
        para.startsWith('<pre')
      ) {
        return para;
      }
      return `<p>${para}</p>`;
    })
    .join('\n');

  return html;
}

function updateTutorialContent() {
  const tutorial = tutorialState.tutorials[tutorialState.currentStep];
  if (!tutorial) return;

  // Update title
  if (tutorialState.dom.title) {
    tutorialState.dom.title.textContent = tutorial.title;
  }

  // Update body content
  if (tutorialState.dom.body) {
    tutorialState.dom.body.innerHTML = renderMarkdown(tutorial.content);
  }

  // Update code in editor (if the editor is available from main.js)
  if (window.playgroundState?.editor && tutorial.code) {
    window.playgroundState.editor.setValue(tutorial.code.trim());
  }

  // Update TOC highlighting
  renderTOC();

  // Update navigation buttons
  updateNavigationButtons();

  // Scroll to top of tutorial content
  const tutorialBottom = document.querySelector('.tutorial-bottom');
  if (tutorialBottom) {
    tutorialBottom.scrollTop = 0;
  }
}

function goToStep(step) {
  if (step < 0 || step >= tutorialState.tutorials.length) return;
  tutorialState.currentStep = step;
  updateTutorialContent();
}

function nextStep() {
  goToStep(tutorialState.currentStep + 1);
}

function prevStep() {
  goToStep(tutorialState.currentStep - 1);
}

async function initTutorial() {
  tutorialState.dom = getDomRefs();

  // Load tutorials
  await loadTutorials();

  if (tutorialState.tutorials.length === 0) {
    if (tutorialState.dom.body) {
      tutorialState.dom.body.innerHTML = '<p>No tutorials available. Please check the tutorials.json file.</p>';
    }
    return;
  }

  // Render TOC
  renderTOC();

  // Set up title dropdown toggle
  if (tutorialState.dom.title) {
    tutorialState.dom.title.addEventListener('click', toggleDropdown);
  }

  // Close dropdown when clicking outside
  document.addEventListener('click', (e) => {
    if (!tutorialState.dom.tocDropdown) return;
    const isClickInside = tutorialState.dom.tocDropdown.contains(e.target) ||
                          tutorialState.dom.title?.contains(e.target);
    if (!isClickInside) {
      closeDropdown();
    }
  });

  // Set up navigation buttons
  if (tutorialState.dom.prevButton) {
    tutorialState.dom.prevButton.addEventListener('click', prevStep);
  }
  if (tutorialState.dom.nextButton) {
    tutorialState.dom.nextButton.addEventListener('click', nextStep);
  }

  // Keyboard navigation
  document.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowLeft' && !tutorialState.dom.prevButton?.disabled) {
      prevStep();
    } else if (e.key === 'ArrowRight' && !tutorialState.dom.nextButton?.disabled) {
      nextStep();
    }
  });

  // Display first tutorial
  updateTutorialContent();
}

// Export state for main.js to access if needed
window.tutorialState = tutorialState;

// Wait for main playground to initialize, then init tutorial
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => {
    // Give main.js a moment to initialize
    setTimeout(initTutorial, 100);
  });
} else {
  setTimeout(initTutorial, 100);
}
