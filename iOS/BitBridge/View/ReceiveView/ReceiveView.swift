//
//  ReceiveView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 19/2/25.
//

import SwiftUI
import Foundation
import SharedTypes

struct ReceiveView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject private var core: Core
    
    @State var receiveSessions: [ReceiveSessionViewModel] = []
    @State var isShowingOptionAll = false
    
    var body: some View {
        ZStack {
            ScrollView {
                LazyVStack(spacing: SpaceTheme.item.value, pinnedViews: [.sectionHeaders]) {
                    LogoScene(gltfFileName: "Rocket", logoScale: 1.5)
                        .frame(width: screenSize.width, height: 100)
                        .overlay(Theme.gradientHeading
                            .opacity(0.5)
                            .blur(radius: 15)
                            .frame(width: .infinity, height: screenSize.width / 2)
                        )
                    
                    Text("Your Inbox")
                        .padding(.horizontal, 20)
                        .multilineTextAlignment(.center)
                        .modifier(Heading2())
                    
                    UpgradePremiumButton()
                    
                    HStack(alignment: .center) {
                        Spacer()
                        Button(action: { isShowingOptionAll = true }) {
                            ImageAsset.More.image
                                .foregroundColor(Theme.PrimaryText.color)
                                .scaleEffect(1.6)
                                .confirmationDialog(
                                    "Options", isPresented: $isShowingOptionAll) {
                                        Button("Select") {}
                                    }
                        }
                        .frame(minWidth: 35, alignment: .trailing)
                    }
                    .padding(.horizontal, SpaceTheme.item.value)
                    
                    ForEach(self.receiveSessions, id: \.self.id) { item in
                        VStack(spacing: SpaceTheme.item.value) {
                            ReceiveSessionHeaderView(session: item)
                                .zIndex(2)
                            
                            ReceiveSessionBodyView(session: item)
                                .zIndex(1)
                        }
                    }
                    .padding(.horizontal, SpaceTheme.screen.value)
                    .padding(.top, SpaceTheme.item.value)
                    Spacer().frame(height: 130)
                }
            }
            .mask(LinearGradient(gradient: Gradient(colors: [.black, .black, .black, .black, .clear]), startPoint: .top, endPoint: .bottom).opacity(0.8))
            .padding(.bottom, SpaceTheme.screen.value)
            
        }
        .onReceive(self.core.transfer, perform: { value in
            let receivedSessions = value?.received_sessions ?? [];
            
            if receivedSessions.count != self.receiveSessions.count {
                self.receiveSessions = receivedSessions
            }
        })
    }
}

#Preview {
    ReceiveView()
        .environmentObject(CoreMock.withSelectedFileTransfers() as Core)

}
