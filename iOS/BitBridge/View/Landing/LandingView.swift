//
//  LandingView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI

struct LandingView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize

    var body: some View {
        ZStack {
            Theme.darkBgGradient
            Theme.starGradient(x: 0.5, y: 0.3)
            VStack(alignment: .leading) {
                LogoView(width: 130)
                    .frame(maxWidth: .infinity)
                    .padding(.bottom, screenSize.height * 0.1)
                Spacer()
                Text("The most secure and fastest file transfer")
                    .font(FontTheme.H1Heading)
                    .modifier(GradientHeading())
                    .padding(.bottom, 17)
                Text("We feel thankful that you’re here ❤️")
                    .foregroundColor(Theme.LightViolet.color)
                    .modifier(Body2())

                Button(action: {}) {
                    Text("Get started")
                }
                .padding(.top, 60)
                .buttonStyle(PrimaryGradientButton(gradient: Theme.primaryGradient))
            }
            .padding(.horizontal, SpaceTheme.screen.value)
            .padding(.bottom, safeAreaInsets.bottom + SpaceTheme.screen.value)
        }
        .ignoresSafeArea(.all)
    }
}

struct LandingView_Preview: PreviewProvider {
    static var previews: some View {
        LandingView()
    }
}
