Why 10KiB?
----------

Why not?

10KiB (10,240 bytes) is small enough to preclude JavaScript framework and web-
font shenanigans, but big enough to produce something useful in my (subjective)
opinion.

Why compressed?
---------------

Text compresses well.  "Media" "assets" do not, so using the compressed size of
the site allows for more content without enabling too much extra overhead.

What Counts?
------------

Any content retrieved when rendering the page, including markup, CSS, JavaScript,
fonts, images, videos, etc.  This includes third-party content.

How is this different than the other *n*-KB website lists?
----------------------------------------------------------

The biggest difference is the default sort order is based on voting, rather than
page size.  I want to showcase useful content and allow people to show off what
they can do within the proscribed size constraint. Maybe, if this gets popular,
we'll even see some clever demo-scene style hacks that fit within 10kb!

The other difference is I wanted to make it easier to submit a site.  Most of the
other clubs require submitting a PR, and while I understand that is a) easy to
implement and b) cuts down on spam, I'd rather make submitting new content easier
on users.  So, here, you can
[submit a URL directly on the site](https://10kb.club/submit.html). **Note: this does
not mean that I'm not going to aggresively combat spam.**

Are there quality standards for the sites submitted?
----------------------------------------------------

Yes. Sites must be more than a contact page, or a text page that says 'This site
is only 26 bytes'. Personal blogs are welcome. Clever hacks are welcome.  A
landing page that just links to much more bloated content is not.  Unlike the
previous 10kb club site run by [Susam Pal](https://susam.net,) there are not any
popularity requirements (such as content being upvoted on
[Hacker News](https://news.ycombinator.com) or [Lobsters](https://lobste.rs).
Like Susam's site, we do *link to* interesting content in a site's profile page.

While listing does not imply endorsement, I reserve the right to not include a
link due to abusive, demeaning, or otherwise inappropriate content.

My site is listed here - will you remove it?
--------------------------------------------

Sure. Email me.

How are sites approved?
-----------------------

1) I run some basic automated checks to determine if the page fits within the
10KiB limit

2) If that passes, then a more thorough check with
[Cloudflare Radar](https://radar.cloudflare.com).

3) If that passes, then I do a final manual review.

**Sites are periodically re-tested to make sure they are still up, and still
meet the size requirement.**
