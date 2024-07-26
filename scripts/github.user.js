// ==UserScript==
// @name         Nixpkgs subscribe
// @namespace    http://tampermonkey.net/
// @version      2024-07-25
// @description  try to take over the world!
// @author       You
// @match        https://github.com/*
// @icon         https://www.google.com/s2/favicons?sz=64&domain=github.com
// @grant        GM_getValue
// ==/UserScript==


function addButton() {
    const buttonContainerElement = document.querySelector('.gh-header-actions');
    const div = document.createElement('div');
    div.classList = 'flex-md-order-2 pr_tracker';
        const pr= window.location.pathname.split('/')[4];

    const btn = document.createElement('button');
    btn.classList = 'Button--secondary Button--small Button';
    btn.type = 'button';
    btn.innerText = `Subscribe`;
    btn.addEventListener('click', function() {
        window.open('https://tracker.lel.lol/?pr='+pr+'&email=' + GM_getValue("email", "example@example.com"), '_blank').focus();
    });


    div.appendChild(btn);
    buttonContainerElement.firstElementChild.before(div);



    const div2 = document.createElement('div');
    div2.classList = 'flex-md-order-2';


    const btn2 = document.createElement('button');
    btn2.classList = 'Button--secondary Button--small Button';
    btn2.type = 'button';
    btn2.innerText = `Track`;
    btn2.addEventListener('click', function() {
        window.open('https://tracker.lel.lol/?pr='+pr, '_blank').focus();
    });

    div2.appendChild(btn2);
    buttonContainerElement.firstElementChild.before(div2);
}

function maybeAddButton(){
    const loc = window.location.pathname;
    if (loc.match("^/NixOS/nixpkgs/pull/[0-9]*") && (document.getElementsByClassName("pr_tracker").length == 0 )){
        addButton();
    }
    setTimeout(maybeAddButton, 250);

}

(function() {
    'use strict';

    // Your code here...
    maybeAddButton();
})();
