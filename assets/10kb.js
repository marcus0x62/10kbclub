/*
 * Copyright (c) 2024 Marcus Butler
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

async function tryvote(site_id, vote) {
    console.log(`vote called with ${site_id} - ${vote}`);
    let url = `/vote/`;

    let voter_id = await get_id(false);
    if (voter_id.length == 0) {
        return; // status text is handled by get_id
    }

    if (vote < 0 || vote > 1) {
        update_status('Invalid vote!');
    }

    let vote_data = new URLSearchParams();
    vote_data.append('site_id', site_id);
    vote_data.append('voter_id', voter_id);
    vote_data.append('vote', vote);

    let json;
    try {
        let res = await fetch(url, { method: 'POST', body: vote_data });
        json = await res.json();
    } catch (error) {
        update_status(`Error casting vote: ${error}`);
    }

    if (json['code'] == 200) {
        if (vote == 1) {
            let elem = document.getElementById(`vote-${site_id}`);
            elem.className = 'upvoted';
            let closure = vote_closure(site_id, 0);
            elem.addEventListener('click', (e) => closure());
        } else if (vote == 0) {
            let elem = document.getElementById(`vote-${site_id}`);
            elem.className = 'unvoted';
            let closure = vote_closure(site_id, 1);
            elem.addEventListener('click', (e) => closure);
        }
    } else {
        update_status(`Unable to vote: ${json['status']}`);
    }
}

async function get_id(force=false) {
    let url = '/id/';

    let id = localStorage.getItem('10kb_voter_id');
    if (id && id.length > 0 && force == false) {
        return id;
    } else {
        let json;

        try {
            let res = await fetch(url, { method: 'POST' });
            json = await res.json();
        } catch (error) {
            update_status(`Error getting poster id: ${error}`);
            return null;
        }

        if (json['code'] == 200) {
            localStorage.setItem('10kb_voter_id', json['voter_id']);
            return json['voter_id'];
        } else {
            update_status(`Unable to generate id: ${json['status']}`);
            return null;
        }
    }
}

async function populate_votes() {
    let url = '/votes/';

    let ids = Array();
    for (elem of document.querySelectorAll("div")) {
        if (elem.id.match(/vote\-/)) {
            let id = elem.id.split('-')[1];
            ids.push(id);
            console.log(`pushing ${id}`);
        }
    }

    for (site_id of ids) {
        let elem = document.getElementById(`vote-${site_id}`);
        elem.className = 'unvoted';
        let closure = vote_closure(site_id, 1);
        elem.addEventListener('click', () => closure());
        elem.innerHTML = '&#10145;';
    }

    let voter_id = localStorage.getItem('10kb_voter_id');

    console.log(`voter id is ${voter_id}`);
    try {
        let json;

        let data = new URLSearchParams();
        data.append('site_ids', ids);
        data.append('voter_id', voter_id);

        let res = await fetch(url, { method: 'POST', body: data });
        json = await res.json();

        for (site_id of json['site_ids']) {
            let elem = document.getElementById(`vote-${site_id}`);
            elem.className = 'upvoted';
            let closure = vote_closure(site_id, 0);
            elem.addEventListener('click', (e) => closure());
        }
    } catch (error) {
        console.log(`Error getting vote data: ${error}`);
        return null;
    }
}

function vote_closure(id, vote) {
    return function() {
        tryvote(id, vote);
    };
}

function update_status(msg) {
    alert(msg);
}

(function () {
    window.addEventListener('DOMContentLoaded', populate_votes, false);
})();
